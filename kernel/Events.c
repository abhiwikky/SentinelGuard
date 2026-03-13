//
// Event processing implementation
//

#include "SentinelGuard.h"
#include "Events.h"
#include "Communication.h"
#include <ntstrsafe.h>

#define SG_EVENT_POOL_TAG 'vgSS'
#define SG_UNIX_EPOCH_DIFFERENCE_100NS 116444736000000000ULL

static ULONGLONG QueryUnixTimestampSeconds(VOID);
static NTSTATUS CaptureNormalizedFilePath(
    _In_ PFLT_CALLBACK_DATA Data,
    _Out_writes_(BufferSize) PWCHAR FilePath,
    _In_ ULONG BufferSize
);
static NTSTATUS CaptureWritePreview(
    _In_ PFLT_CALLBACK_DATA Data,
    _Out_writes_all_(sizeof(((PFILE_EVENT)0)->EntropyPreview)) UCHAR Preview[16]
);

VOID ProcessFileEvent(
    _In_ PFLT_CALLBACK_DATA Data,
    _In_ SG_EVENT_TYPE EventType
)
{
    PSG_EVENT_CONTEXT eventContext = NULL;
    FILE_EVENT event = { 0 };

    if (!NT_SUCCESS(CaptureEventContext(Data, EventType, &eventContext))) {
        return;
    }

    BuildFileEventFromContext(Data, eventContext, &event);
    (VOID)SendEventToUserMode(&event);
    FreeEventContext(eventContext);
}

NTSTATUS CaptureEventContext(
    _In_ PFLT_CALLBACK_DATA Data,
    _In_ SG_EVENT_TYPE EventType,
    _Outptr_ PSG_EVENT_CONTEXT *EventContext
)
{
    NTSTATUS status;
    PSG_EVENT_CONTEXT context;

    if (!EventContext) {
        return STATUS_INVALID_PARAMETER;
    }

    *EventContext = NULL;

    context = ExAllocatePoolZero(NonPagedPoolNx, sizeof(SG_EVENT_CONTEXT), SG_EVENT_POOL_TAG);
    if (!context) {
        return STATUS_INSUFFICIENT_RESOURCES;
    }

    context->EventType = EventType;
    context->ProcessId = (ULONG)(ULONG_PTR)FltGetRequestorProcessId(Data);

    status = GetProcessPath(
        context->ProcessId,
        context->ProcessPath,
        RTL_NUMBER_OF(context->ProcessPath)
    );
    if (!NT_SUCCESS(status)) {
        RtlZeroMemory(context->ProcessPath, sizeof(context->ProcessPath));
    }

    status = CaptureNormalizedFilePath(
        Data,
        context->FilePath,
        RTL_NUMBER_OF(context->FilePath)
    );
    if (!NT_SUCCESS(status)) {
        RtlZeroMemory(context->FilePath, sizeof(context->FilePath));
    }

    if (EventType == EventFileWrite) {
        (VOID)CaptureWritePreview(Data, context->EntropyPreview);
    }

    *EventContext = context;
    return STATUS_SUCCESS;
}

VOID BuildFileEventFromContext(
    _In_ PFLT_CALLBACK_DATA Data,
    _In_ PSG_EVENT_CONTEXT EventContext,
    _Out_ PFILE_EVENT Event
)
{
    ULONG_PTR transferredBytes;

    RtlZeroMemory(Event, sizeof(FILE_EVENT));

    Event->Type = EventContext->EventType;
    Event->ProcessId = EventContext->ProcessId;
    Event->Timestamp = QueryUnixTimestampSeconds();
    Event->Result = (ULONG)Data->IoStatus.Status;

    RtlCopyMemory(Event->ProcessPath, EventContext->ProcessPath, sizeof(Event->ProcessPath));
    RtlCopyMemory(Event->FilePath, EventContext->FilePath, sizeof(Event->FilePath));
    RtlCopyMemory(Event->EntropyPreview, EventContext->EntropyPreview, sizeof(Event->EntropyPreview));

    transferredBytes = NT_SUCCESS(Data->IoStatus.Status) ? Data->IoStatus.Information : 0;

    if (EventContext->EventType == EventFileRead) {
        Event->BytesRead = (ULONGLONG)transferredBytes;
    } else if (EventContext->EventType == EventFileWrite) {
        Event->BytesWritten = (ULONGLONG)transferredBytes;
    }
}

VOID FreeEventContext(
    _In_opt_ PSG_EVENT_CONTEXT EventContext
)
{
    if (EventContext) {
        ExFreePoolWithTag(EventContext, SG_EVENT_POOL_TAG);
    }
}

NTSTATUS GetProcessPath(
    _In_ ULONG ProcessId,
    _Out_ PWCHAR ProcessPath,
    _In_ ULONG BufferSize
)
{
    NTSTATUS status;
    PEPROCESS process = NULL;
    PUNICODE_STRING imagePath = NULL;

    status = PsLookupProcessByProcessId(ULongToHandle(ProcessId), &process);
    if (!NT_SUCCESS(status)) {
        return status;
    }

    status = SeLocateProcessImageName(process, &imagePath);
    if (NT_SUCCESS(status) && imagePath) {
        ULONG copySize = min(BufferSize - 1, imagePath->Length / sizeof(WCHAR));
        RtlCopyMemory(ProcessPath, imagePath->Buffer, copySize * sizeof(WCHAR));
        ProcessPath[copySize] = L'\0';
    }

    if (imagePath) {
        ExFreePool(imagePath);
    }

    ObDereferenceObject(process);
    return status;
}

NTSTATUS GetFilePath(
    _In_ PFLT_FILE_NAME_INFORMATION FileNameInfo,
    _Out_ PWCHAR FilePath,
    _In_ ULONG BufferSize
)
{
    ULONG copySize;

    if (!FileNameInfo || !FileNameInfo->Name.Buffer || BufferSize == 0) {
        return STATUS_INVALID_PARAMETER;
    }

    copySize = min(BufferSize - 1, FileNameInfo->Name.Length / sizeof(WCHAR));
    RtlCopyMemory(FilePath, FileNameInfo->Name.Buffer, copySize * sizeof(WCHAR));
    FilePath[copySize] = L'\0';

    return STATUS_SUCCESS;
}

UCHAR CalculateEntropy(
    _In_ PUCHAR Data,
    _In_ ULONG Length
)
{
    ULONG frequency[256] = { 0 };
    ULONG i;
    ULONG distinctValues = 0;

    if (!Data || Length == 0) {
        return 0;
    }

    for (i = 0; i < Length; i++) {
        frequency[Data[i]]++;
    }

    for (i = 0; i < RTL_NUMBER_OF(frequency); i++) {
        if (frequency[i] > 0) {
            distinctValues++;
        }
    }

    return (UCHAR)min(255, distinctValues);
}

static ULONGLONG QueryUnixTimestampSeconds(VOID)
{
    LARGE_INTEGER systemTime;
    ULONGLONG windowsTicks;

    KeQuerySystemTimePrecise(&systemTime);
    windowsTicks = (ULONGLONG)systemTime.QuadPart;

    if (windowsTicks <= SG_UNIX_EPOCH_DIFFERENCE_100NS) {
        return 0;
    }

    return (windowsTicks - SG_UNIX_EPOCH_DIFFERENCE_100NS) / 10000000ULL;
}

static NTSTATUS CaptureNormalizedFilePath(
    _In_ PFLT_CALLBACK_DATA Data,
    _Out_writes_(BufferSize) PWCHAR FilePath,
    _In_ ULONG BufferSize
)
{
    NTSTATUS status;
    PFLT_FILE_NAME_INFORMATION nameInfo = NULL;

    status = FltGetFileNameInformation(
        Data,
        FLT_FILE_NAME_NORMALIZED | FLT_FILE_NAME_QUERY_DEFAULT,
        &nameInfo
    );

    if (!NT_SUCCESS(status)) {
        return status;
    }

    status = FltParseFileNameInformation(nameInfo);
    if (NT_SUCCESS(status)) {
        status = GetFilePath(nameInfo, FilePath, BufferSize);
    }

    FltReleaseFileNameInformation(nameInfo);
    return status;
}

static NTSTATUS CaptureWritePreview(
    _In_ PFLT_CALLBACK_DATA Data,
    _Out_writes_all_(16) UCHAR Preview[16]
)
{
    NTSTATUS status = STATUS_SUCCESS;
    PFLT_IO_PARAMETER_BLOCK iopb;
    PMDL mdl;
    PUCHAR buffer = NULL;
    ULONG bytesToCopy;

    RtlZeroMemory(Preview, 16);

    iopb = Data->Iopb;
    if (!iopb || iopb->MajorFunction != IRP_MJ_WRITE || iopb->Parameters.Write.Length == 0) {
        return STATUS_SUCCESS;
    }

    mdl = iopb->Parameters.Write.MdlAddress;
    if (!mdl && Data->RequestorMode != KernelMode) {
        status = FltLockUserBuffer(Data);
        if (!NT_SUCCESS(status)) {
            return status;
        }
        mdl = iopb->Parameters.Write.MdlAddress;
    }

    if (mdl) {
        buffer = MmGetSystemAddressForMdlSafe(mdl, NormalPagePriority | MdlMappingNoExecute);
        if (!buffer) {
            return STATUS_INSUFFICIENT_RESOURCES;
        }
    } else {
        buffer = (PUCHAR)iopb->Parameters.Write.WriteBuffer;
    }

    if (!buffer) {
        return STATUS_INVALID_USER_BUFFER;
    }

    bytesToCopy = min((ULONG)sizeof(((PFILE_EVENT)0)->EntropyPreview), iopb->Parameters.Write.Length);

    __try {
        RtlCopyMemory(Preview, buffer, bytesToCopy);
    } __except (EXCEPTION_EXECUTE_HANDLER) {
        status = GetExceptionCode();
    }

    return status;
}
