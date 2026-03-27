/*
 * SentinelGuard Minifilter Driver - Operation Callbacks
 *
 * Pre-operation callbacks for intercepting file system operations.
 * Each callback constructs an SG_EVENT and sends it to user mode.
 *
 * Performance: SgShouldExclude() filters out noise BEFORE the
 * expensive SgPopulateEvent() call (which queries file names and
 * process image names).  This eliminates ~90% of background I/O.
 */

#include "driver.h"

/* ─── Exclusion Lists (Kernel-Side Filtering) ──────────────────────── */

/* File extensions that are never ransomware targets.
 * Checked case-insensitively against the final name component. */
static const UNICODE_STRING g_ExcludedExtensions[] = {
    RTL_CONSTANT_STRING(L".log"),
    RTL_CONSTANT_STRING(L".tmp"),
    RTL_CONSTANT_STRING(L".etl"),
    RTL_CONSTANT_STRING(L".pf"),
    RTL_CONSTANT_STRING(L".db-journal"),
    RTL_CONSTANT_STRING(L".db-wal"),
    RTL_CONSTANT_STRING(L".lnk"),
    RTL_CONSTANT_STRING(L".sys"),
    RTL_CONSTANT_STRING(L".cat"),
    RTL_CONSTANT_STRING(L".mui"),
    RTL_CONSTANT_STRING(L".nls"),
};

#define SG_EXCLUDED_EXT_COUNT (sizeof(g_ExcludedExtensions) / sizeof(g_ExcludedExtensions[0]))

/* Path prefixes that are pure OS noise. Compared case-insensitively. */
static const UNICODE_STRING g_ExcludedPathPrefixes[] = {
    RTL_CONSTANT_STRING(L"\\Windows\\Prefetch\\"),
    RTL_CONSTANT_STRING(L"\\Windows\\Temp\\"),
    RTL_CONSTANT_STRING(L"\\Windows\\Logs\\"),
    RTL_CONSTANT_STRING(L"\\Windows\\System32\\LogFiles\\"),
    RTL_CONSTANT_STRING(L"\\Windows\\System32\\winevt\\"),
    RTL_CONSTANT_STRING(L"\\$Recycle.Bin\\"),
    RTL_CONSTANT_STRING(L"\\System Volume Information\\"),
    RTL_CONSTANT_STRING(L"\\ProgramData\\SentinelGuard\\logs\\"),
};

#define SG_EXCLUDED_PATH_COUNT (sizeof(g_ExcludedPathPrefixes) / sizeof(g_ExcludedPathPrefixes[0]))

/*
 * SgShouldExclude – Lightweight pre-filter run BEFORE SgPopulateEvent.
 *
 * Returns TRUE if this I/O should be silently ignored.
 * Uses only data available cheaply from the callback parameters
 * (file name information from the FO / Data parameters and PID).
 */
static BOOLEAN
SgShouldExclude(
    _In_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects
)
{
    NTSTATUS status;
    PFLT_FILE_NAME_INFORMATION nameInfo = NULL;
    ULONG i;
    BOOLEAN exclude = FALSE;

    UNREFERENCED_PARAMETER(FltObjects);

    /* 1. Skip events from our own agent process (avoids feedback loop) */
    /* -- we cannot cheaply know the agent PID in kernel, skip this check -- */

    /* 2. Attempt a cheap name query (opened or short name).
     * If we can't get the name cheaply, let the event through. */
    status = FltGetFileNameInformation(
        Data,
        FLT_FILE_NAME_NORMALIZED | FLT_FILE_NAME_QUERY_DEFAULT,
        &nameInfo
    );

    if (!NT_SUCCESS(status) || nameInfo == NULL) {
        return FALSE;
    }

    status = FltParseFileNameInformation(nameInfo);
    if (!NT_SUCCESS(status)) {
        FltReleaseFileNameInformation(nameInfo);
        return FALSE;
    }

    /* 3. Check extension exclusion list */
    if (nameInfo->Extension.Length > 0) {
        for (i = 0; i < SG_EXCLUDED_EXT_COUNT; i++) {
            if (RtlEqualUnicodeString(
                    &nameInfo->Extension,
                    &g_ExcludedExtensions[i],
                    TRUE /* case-insensitive */)) {
                exclude = TRUE;
                break;
            }
        }
    }

    /* 4. Check path prefix exclusion list */
    if (!exclude && nameInfo->Name.Length > 0) {
        for (i = 0; i < SG_EXCLUDED_PATH_COUNT; i++) {
            /* RtlPrefixUnicodeString checks if g_ExcludedPathPrefixes[i]
             * is a prefix of nameInfo->Name (case-insensitive). 
             * We search for the prefix anywhere in the full path since
             * the volume prefix varies (e.g. \Device\HarddiskVolume3\...) */
            if (nameInfo->Name.Length >= g_ExcludedPathPrefixes[i].Length) {
                UNICODE_STRING suffix;
                USHORT offset;
                BOOLEAN found = FALSE;

                /* Scan for the prefix substring within the name */
                for (offset = 0;
                     offset <= (nameInfo->Name.Length - g_ExcludedPathPrefixes[i].Length) / sizeof(WCHAR);
                     offset++) {
                    suffix.Buffer = nameInfo->Name.Buffer + offset;
                    suffix.Length = g_ExcludedPathPrefixes[i].Length;
                    suffix.MaximumLength = suffix.Length;
                    if (RtlEqualUnicodeString(&suffix, &g_ExcludedPathPrefixes[i], TRUE)) {
                        found = TRUE;
                        break;
                    }
                }
                if (found) {
                    exclude = TRUE;
                    break;
                }
            }
        }
    }

    FltReleaseFileNameInformation(nameInfo);
    return exclude;
}

/* ─── Utility: Get Process Image Name ─────────────────────────────────── */

VOID
SgGetProcessName(
    _In_ PEPROCESS Process,
    _Out_writes_(MaxLength) PWCHAR Buffer,
    _In_ ULONG MaxLength
)
{
    NTSTATUS status;
    PUNICODE_STRING processName = NULL;
    ULONG returnedLength = 0;

    RtlZeroMemory(Buffer, MaxLength * sizeof(WCHAR));

    status = SeLocateProcessImageName(Process, &processName);
    if (NT_SUCCESS(status) && processName != NULL && processName->Buffer != NULL) {
        ULONG copyLen = processName->Length / sizeof(WCHAR);
        if (copyLen >= MaxLength) {
            copyLen = MaxLength - 1;
        }
        RtlCopyMemory(Buffer, processName->Buffer, copyLen * sizeof(WCHAR));
        Buffer[copyLen] = L'\0';
        ExFreePool(processName);
    } else {
        RtlStringCchCopyW(Buffer, MaxLength, L"<unknown>");
    }
}

/* ─── Utility: Extract File Extension ─────────────────────────────────── */

VOID
SgGetFileExtension(
    _In_ PUNICODE_STRING FileName,
    _Out_writes_(MaxLength) PWCHAR Buffer,
    _In_ ULONG MaxLength
)
{
    LONG i;

    RtlZeroMemory(Buffer, MaxLength * sizeof(WCHAR));

    if (FileName == NULL || FileName->Buffer == NULL || FileName->Length == 0) {
        return;
    }

    /* Walk backward from end of filename to find last '.' */
    for (i = (FileName->Length / sizeof(WCHAR)) - 1; i >= 0; i--) {
        if (FileName->Buffer[i] == L'.') {
            ULONG extLen = (FileName->Length / sizeof(WCHAR)) - i;
            if (extLen >= MaxLength) {
                extLen = MaxLength - 1;
            }
            RtlCopyMemory(Buffer, &FileName->Buffer[i], extLen * sizeof(WCHAR));
            Buffer[extLen] = L'\0';
            return;
        }
        /* Stop at path separators */
        if (FileName->Buffer[i] == L'\\' || FileName->Buffer[i] == L'/') {
            break;
        }
    }
}

/* ─── Utility: Populate Event Structure ───────────────────────────────── */

VOID
SgPopulateEvent(
    _Out_ PSG_EVENT Event,
    _In_ SG_OPERATION_TYPE OpType,
    _In_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects
)
{
    PFLT_FILE_NAME_INFORMATION nameInfo = NULL;
    NTSTATUS status;
    PEPROCESS process;

    RtlZeroMemory(Event, sizeof(SG_EVENT));
    Event->StructSize = sizeof(SG_EVENT);
    Event->Operation = OpType;
    Event->ProcessId = (ULONG)(ULONG_PTR)PsGetCurrentProcessId();

    /* Precise timestamp */
    KeQuerySystemTimePrecise(&Event->Timestamp);

    /* Get file name information */
    status = FltGetFileNameInformation(
        Data,
        FLT_FILE_NAME_NORMALIZED | FLT_FILE_NAME_QUERY_DEFAULT,
        &nameInfo
    );

    if (NT_SUCCESS(status)) {
        status = FltParseFileNameInformation(nameInfo);
        if (NT_SUCCESS(status)) {
            /* Copy full file path */
            ULONG copyLen = nameInfo->Name.Length / sizeof(WCHAR);
            if (copyLen >= SG_MAX_PATH_LENGTH) {
                copyLen = SG_MAX_PATH_LENGTH - 1;
            }
            RtlCopyMemory(Event->FilePath, nameInfo->Name.Buffer,
                copyLen * sizeof(WCHAR));
            Event->FilePath[copyLen] = L'\0';

            /* Extract extension */
            SgGetFileExtension(&nameInfo->Extension, Event->FileExtension, 32);
        }
        FltReleaseFileNameInformation(nameInfo);
    }

    /* Get process name */
    process = PsGetCurrentProcess();
    if (process != NULL) {
        SgGetProcessName(process, Event->ProcessName, SG_MAX_PROCESS_NAME);
    }

    UNREFERENCED_PARAMETER(FltObjects);
}

/* ─── Pre-Create Callback ─────────────────────────────────────────────── */

FLT_PREOP_CALLBACK_STATUS
SgPreCreateCallback(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
)
{
    SG_EVENT event;
    ULONG createDisposition;

    *CompletionContext = NULL;

    /* Skip kernel-mode requests and paging I/O */
    if (Data->RequestorMode == KernelMode) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    /* Kernel-side noise filter: skip excluded extensions/paths */
    if (SgShouldExclude(Data, FltObjects)) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    /* Check if this is a delete-on-close */
    createDisposition = (Data->Iopb->Parameters.Create.Options >> 24) & 0xFF;

    if (Data->Iopb->Parameters.Create.Options & FILE_DELETE_ON_CLOSE) {
        SgPopulateEvent(&event, SgOpDelete, Data, FltObjects);
    } else {
        SgPopulateEvent(&event, SgOpCreate, Data, FltObjects);
    }

    SgSendEvent(&event);

    return FLT_PREOP_SUCCESS_NO_CALLBACK;
}

/* ─── Pre-Write Callback ──────────────────────────────────────────────── */

FLT_PREOP_CALLBACK_STATUS
SgPreWriteCallback(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
)
{
    SG_EVENT event;

    *CompletionContext = NULL;

    /* Skip kernel-mode and paging I/O */
    if (Data->RequestorMode == KernelMode ||
        FlagOn(Data->Iopb->IrpFlags, IRP_PAGING_IO)) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    /* Kernel-side noise filter */
    if (SgShouldExclude(Data, FltObjects)) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    SgPopulateEvent(&event, SgOpWrite, Data, FltObjects);
    event.FileSize = (LONGLONG)Data->Iopb->Parameters.Write.Length;

    SgSendEvent(&event);

    return FLT_PREOP_SUCCESS_NO_CALLBACK;
}

/* ─── Pre-SetInformation Callback (rename & delete) ───────────────────── */

FLT_PREOP_CALLBACK_STATUS
SgPreSetInfoCallback(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
)
{
    SG_EVENT event;
    FILE_INFORMATION_CLASS infoClass;

    *CompletionContext = NULL;

    if (Data->RequestorMode == KernelMode) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    /* Kernel-side noise filter */
    if (SgShouldExclude(Data, FltObjects)) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    infoClass = Data->Iopb->Parameters.SetFileInformation.FileInformationClass;

    if (infoClass == FileRenameInformation ||
        infoClass == FileRenameInformationEx) {

        SgPopulateEvent(&event, SgOpRename, Data, FltObjects);

        /* Try to capture the new name from the rename info buffer */
        {
            PFILE_RENAME_INFORMATION renameInfo =
                (PFILE_RENAME_INFORMATION)Data->Iopb->Parameters.SetFileInformation.InfoBuffer;

            if (renameInfo != NULL && renameInfo->FileNameLength > 0) {
                ULONG copyLen = renameInfo->FileNameLength / sizeof(WCHAR);
                if (copyLen >= SG_MAX_PATH_LENGTH) {
                    copyLen = SG_MAX_PATH_LENGTH - 1;
                }
                RtlCopyMemory(event.NewFilePath, renameInfo->FileName,
                    copyLen * sizeof(WCHAR));
                event.NewFilePath[copyLen] = L'\0';
            }
        }

        SgSendEvent(&event);

    } else if (infoClass == FileDispositionInformation ||
               infoClass == FileDispositionInformationEx) {

        SgPopulateEvent(&event, SgOpDelete, Data, FltObjects);
        SgSendEvent(&event);
    }

    return FLT_PREOP_SUCCESS_NO_CALLBACK;
}

/* ─── Pre-DirectoryControl Callback ───────────────────────────────────── */

FLT_PREOP_CALLBACK_STATUS
SgPreDirCtrlCallback(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
)
{
    SG_EVENT event;

    *CompletionContext = NULL;

    if (Data->RequestorMode == KernelMode) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    /* Kernel-side noise filter */
    if (SgShouldExclude(Data, FltObjects)) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    /* Only intercept IRP_MN_QUERY_DIRECTORY */
    if (Data->Iopb->MinorFunction != IRP_MN_QUERY_DIRECTORY) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    SgPopulateEvent(&event, SgOpDirectoryEnum, Data, FltObjects);
    SgSendEvent(&event);

    return FLT_PREOP_SUCCESS_NO_CALLBACK;
}
