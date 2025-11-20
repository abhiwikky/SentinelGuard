//
// SentinelGuard Kernel Minifilter Driver
// Main driver file
//

#include "SentinelGuard.h"
#include "Events.h"
#include "Communication.h"

// Global driver context
DRIVER_CONTEXT g_DriverContext = { 0 };

// Filter registration structure
const FLT_OPERATION_REGISTRATION Callbacks[] = {
    { IRP_MJ_CREATE,
      0,
      SentinelGuardPreOperation,
      SentinelGuardPostOperation },
    { IRP_MJ_WRITE,
      0,
      SentinelGuardPreOperation,
      SentinelGuardPostOperation },
    { IRP_MJ_READ,
      0,
      SentinelGuardPreOperation,
      SentinelGuardPostOperation },
    { IRP_MJ_SET_INFORMATION,
      0,
      SentinelGuardPreOperation,
      SentinelGuardPostOperation },
    { IRP_MJ_OPERATION_END }
};

const FLT_REGISTRATION FilterRegistration = {
    sizeof(FLT_REGISTRATION),
    FLT_REGISTRATION_VERSION,
    0,
    NULL,
    Callbacks,
    SentinelGuardUnload,
    SentinelGuardInstanceQueryTeardown,
    SentinelGuardInstanceTeardownStart,
    SentinelGuardInstanceTeardownComplete,
    SentinelGuardInstanceSetup,
    NULL,
    NULL,
    NULL
};

NTSTATUS DriverEntry(
    _In_ PDRIVER_OBJECT DriverObject,
    _In_ PUNICODE_STRING RegistryPath
)
{
    NTSTATUS status;
    UNREFERENCED_PARAMETER(RegistryPath);

    // Register minifilter
    status = FltRegisterFilter(
        DriverObject,
        &FilterRegistration,
        &g_DriverContext.FilterHandle
    );

    if (!NT_SUCCESS(status)) {
        return status;
    }

    // Create communication port
    status = CreateCommunicationPort();
    if (!NT_SUCCESS(status)) {
        FltUnregisterFilter(g_DriverContext.FilterHandle);
        return status;
    }

    // Start filtering
    status = FltStartFiltering(g_DriverContext.FilterHandle);
    if (!NT_SUCCESS(status)) {
        CloseCommunicationPort();
        FltUnregisterFilter(g_DriverContext.FilterHandle);
        return status;
    }

    return STATUS_SUCCESS;
}

NTSTATUS SentinelGuardUnload(
    _In_ FLT_FILTER_UNLOAD_FLAGS Flags
)
{
    UNREFERENCED_PARAMETER(Flags);

    CloseCommunicationPort();
    FltUnregisterFilter(g_DriverContext.FilterHandle);

    return STATUS_SUCCESS;
}

NTSTATUS SentinelGuardInstanceQueryTeardown(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_QUERY_TEARDOWN_FLAGS Flags
)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(Flags);
    return STATUS_SUCCESS;
}

VOID SentinelGuardInstanceTeardownStart(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_TEARDOWN_FLAGS Flags
)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(Flags);
}

VOID SentinelGuardInstanceTeardownComplete(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_TEARDOWN_FLAGS Flags
)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(Flags);
}

NTSTATUS SentinelGuardInstanceSetup(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_SETUP_FLAGS Flags,
    _In_ DEVICE_TYPE VolumeDeviceType,
    _In_ FLT_FILESYSTEM_TYPE VolumeFilesystemType
)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(Flags);
    UNREFERENCED_PARAMETER(VolumeDeviceType);
    UNREFERENCED_PARAMETER(VolumeFilesystemType);
    return STATUS_SUCCESS;
}

FLT_PREOP_CALLBACK_STATUS SentinelGuardPreOperation(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(CompletionContext);

    PFLT_IO_PARAMETER_BLOCK iopb = Data->Iopb;
    EVENT_TYPE eventType;

    // Determine event type
    switch (iopb->MajorFunction) {
    case IRP_MJ_CREATE:
        eventType = EventFileCreate;
        break;
    case IRP_MJ_WRITE:
        eventType = EventFileWrite;
        break;
    case IRP_MJ_READ:
        eventType = EventFileRead;
        break;
    case IRP_MJ_SET_INFORMATION:
        if (iopb->Parameters.SetFileInformation.FileInformationClass == FileRenameInformation ||
            iopb->Parameters.SetFileInformation.FileInformationClass == FileRenameInformationEx) {
            eventType = EventFileRename;
        } else if (iopb->Parameters.SetFileInformation.FileInformationClass == FileDispositionInformation) {
            eventType = EventFileDelete;
        } else {
            return FLT_PREOP_SUCCESS_WITH_CALLBACK;
        }
        break;
    default:
        return FLT_PREOP_SUCCESS_WITH_CALLBACK;
    }

    // Process event asynchronously
    ProcessFileEvent(Data, eventType);

    return FLT_PREOP_SUCCESS_WITH_CALLBACK;
}

FLT_POSTOP_CALLBACK_STATUS SentinelGuardPostOperation(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_opt_ PVOID CompletionContext,
    _In_ FLT_POST_OPERATION_FLAGS Flags
)
{
    UNREFERENCED_PARAMETER(Data);
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(CompletionContext);
    UNREFERENCED_PARAMETER(Flags);

    return FLT_POSTOP_FINISHED_PROCESSING;
}

