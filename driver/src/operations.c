/*
 * SentinelGuard Minifilter Driver - Operation Callbacks
 *
 * Pre-operation callbacks for intercepting file system operations.
 * Each callback constructs an SG_EVENT and sends it to user mode.
 */

#include "driver.h"

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

    /* Only intercept IRP_MN_QUERY_DIRECTORY */
    if (Data->Iopb->MinorFunction != IRP_MN_QUERY_DIRECTORY) {
        return FLT_PREOP_SUCCESS_NO_CALLBACK;
    }

    SgPopulateEvent(&event, SgOpDirectoryEnum, Data, FltObjects);
    SgSendEvent(&event);

    return FLT_PREOP_SUCCESS_NO_CALLBACK;
}
