//
// SentinelGuard Kernel Minifilter Driver
// Main header file
//

#pragma once

#include <fltKernel.h>
#include <dontuse.h>
#include <suppress.h>

// Driver version
#define SENTINELGUARD_VERSION_MAJOR 1
#define SENTINELGUARD_VERSION_MINOR 0
#define SENTINELGUARD_VERSION_BUILD 0

// Driver name
#define SENTINELGUARD_DRIVER_NAME L"SentinelGuard"

// Communication port name
#define SENTINELGUARD_PORT_NAME L"\\SentinelGuardPort"

// Maximum message size
#define MAX_MESSAGE_SIZE 4096

// Event types
typedef enum _SG_EVENT_TYPE {
    EventFileCreate,
    EventFileRead,
    EventFileWrite,
    EventFileRename,
    EventFileDelete,
    EventDirectoryEnum,
    EventVSSDelete,
    EventProcessCreate,
    EventRegistryChange
} SG_EVENT_TYPE;

// Event structure sent to user-mode
typedef struct _FILE_EVENT {
    SG_EVENT_TYPE Type;
    ULONG ProcessId;
    WCHAR ProcessPath[512];
    WCHAR FilePath[1024];
    ULONGLONG BytesRead;
    ULONGLONG BytesWritten;
    ULONGLONG Timestamp;
    ULONG Result;
    UCHAR EntropyPreview[16];  // First 16 bytes for entropy calculation
} FILE_EVENT, *PFILE_EVENT;

// Driver context
typedef struct _DRIVER_CONTEXT {
    PFLT_FILTER FilterHandle;
    PFLT_PORT ServerPort;
    PFLT_PORT ClientPort;
} DRIVER_CONTEXT, *PDRIVER_CONTEXT;

extern DRIVER_CONTEXT g_DriverContext;

// Function declarations
NTSTATUS DriverEntry(
    _In_ PDRIVER_OBJECT DriverObject,
    _In_ PUNICODE_STRING RegistryPath
);

NTSTATUS SentinelGuardUnload(
    _In_ FLT_FILTER_UNLOAD_FLAGS Flags
);

NTSTATUS SentinelGuardInstanceQueryTeardown(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_QUERY_TEARDOWN_FLAGS Flags
);

VOID SentinelGuardInstanceTeardownStart(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_TEARDOWN_FLAGS Flags
);

VOID SentinelGuardInstanceTeardownComplete(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_TEARDOWN_FLAGS Flags
);

NTSTATUS SentinelGuardInstanceSetup(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_SETUP_FLAGS Flags,
    _In_ DEVICE_TYPE VolumeDeviceType,
    _In_ FLT_FILESYSTEM_TYPE VolumeFilesystemType
);

// Filter operation callbacks
FLT_PREOP_CALLBACK_STATUS SentinelGuardPreOperation(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
);

FLT_POSTOP_CALLBACK_STATUS SentinelGuardPostOperation(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_opt_ PVOID CompletionContext,
    _In_ FLT_POST_OPERATION_FLAGS Flags
);

// Communication functions
NTSTATUS CreateCommunicationPort(VOID);
VOID CloseCommunicationPort(VOID);
NTSTATUS SendEventToUserMode(_In_ PFILE_EVENT Event);

// Event processing
VOID ProcessFileEvent(
    _In_ PFLT_CALLBACK_DATA Data,
    _In_ SG_EVENT_TYPE EventType
);

// Utility functions
NTSTATUS GetProcessPath(
    _In_ ULONG ProcessId,
    _Out_ PWCHAR ProcessPath,
    _In_ ULONG BufferSize
);

NTSTATUS GetFilePath(
    _In_ PFLT_FILE_NAME_INFORMATION FileNameInfo,
    _Out_ PWCHAR FilePath,
    _In_ ULONG BufferSize
);

UCHAR CalculateEntropy(
    _In_ PUCHAR Data,
    _In_ ULONG Length
);

