#ifndef SENTINELGUARD_DRIVER_H
#define SENTINELGUARD_DRIVER_H

#include <fltKernel.h>
#include <dontuse.h>
#include <suppress.h>
#include <ntstrsafe.h>

/* ─── Configuration Constants ──────────────────────────────────────────── */

#define SG_DRIVER_TAG             'dGsS'  /* SsGd - pool tag */
#define SG_PORT_NAME              L"\\SentinelGuardPort"
#define SG_MAX_PATH_LENGTH        520     /* WCHAR count for file paths */
#define SG_MAX_PROCESS_NAME       260     /* WCHAR count for process name */
#define SG_MAX_CONNECTIONS        1
#define SG_MAX_MESSAGE_SIZE       65536

/* Altitude: 370000 range is for anti-virus like filters */
#define SG_ALTITUDE               L"370050"

/* ─── Operation Types ──────────────────────────────────────────────────── */

typedef enum _SG_OPERATION_TYPE {
    SgOpUnknown = 0,
    SgOpCreate = 1,
    SgOpWrite = 2,
    SgOpRead = 3,
    SgOpRename = 4,
    SgOpDelete = 5,
    SgOpDirectoryEnum = 6,
    SgOpShadowCopyDelete = 7
} SG_OPERATION_TYPE;

/* ─── Kernel Event Structure ───────────────────────────────────────────── */
/* This is the fixed-size event sent from kernel to user mode via the
   communication port. Must be kept in sync with the Rust agent's
   corresponding struct definition. */

#pragma pack(push, 8)
typedef struct _SG_EVENT {
    ULONG             StructSize;                          /* sizeof(SG_EVENT) for versioning */
    SG_OPERATION_TYPE Operation;
    ULONG             ProcessId;
    LARGE_INTEGER     Timestamp;                           /* KeQuerySystemTimePrecise */
    LONGLONG          FileSize;
    WCHAR             FilePath[SG_MAX_PATH_LENGTH];
    WCHAR             NewFilePath[SG_MAX_PATH_LENGTH];     /* Used for rename ops */
    WCHAR             ProcessName[SG_MAX_PROCESS_NAME];
    WCHAR             FileExtension[32];
} SG_EVENT, *PSG_EVENT;
#pragma pack(pop)

/* ─── Message Header for FltSendMessage ────────────────────────────────── */

typedef struct _SG_MESSAGE {
    FILTER_MESSAGE_HEADER Header;
    SG_EVENT Event;
} SG_MESSAGE, *PSG_MESSAGE;

/* ─── Global Data ─────────────────────────────────────────────────────── */

typedef struct _SG_DRIVER_DATA {
    PFLT_FILTER       FilterHandle;
    PFLT_PORT         ServerPort;
    PFLT_PORT         ClientPort;
    BOOLEAN           ClientConnected;
    PDEVICE_OBJECT    DeviceObject;
} SG_DRIVER_DATA, *PSG_DRIVER_DATA;

extern SG_DRIVER_DATA g_DriverData;

/* ─── Communication Functions ─────────────────────────────────────────── */

NTSTATUS
SgCreateCommunicationPort(
    _In_ PFLT_FILTER FilterHandle
);

VOID
SgCloseCommunicationPort(VOID);

NTSTATUS
SgSendEvent(
    _In_ PSG_EVENT Event
);

/* Communication port callbacks */
NTSTATUS
SgPortConnect(
    _In_ PFLT_PORT ClientPort,
    _In_opt_ PVOID ServerPortCookie,
    _In_reads_bytes_opt_(SizeOfContext) PVOID ConnectionContext,
    _In_ ULONG SizeOfContext,
    _Outptr_result_maybenull_ PVOID *ConnectionPortCookie
);

VOID
SgPortDisconnect(
    _In_opt_ PVOID ConnectionCookie
);

/* ─── Operation Callbacks ─────────────────────────────────────────────── */

FLT_PREOP_CALLBACK_STATUS
SgPreCreateCallback(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
);

FLT_PREOP_CALLBACK_STATUS
SgPreWriteCallback(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
);

FLT_PREOP_CALLBACK_STATUS
SgPreSetInfoCallback(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
);

FLT_PREOP_CALLBACK_STATUS
SgPreDirCtrlCallback(
    _Inout_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _Flt_CompletionContext_Outptr_ PVOID *CompletionContext
);

/* ─── Utility Functions ───────────────────────────────────────────────── */

VOID
SgPopulateEvent(
    _Out_ PSG_EVENT Event,
    _In_ SG_OPERATION_TYPE OpType,
    _In_ PFLT_CALLBACK_DATA Data,
    _In_ PCFLT_RELATED_OBJECTS FltObjects
);

VOID
SgGetProcessName(
    _In_ PEPROCESS Process,
    _Out_writes_(MaxLength) PWCHAR Buffer,
    _In_ ULONG MaxLength
);

VOID
SgGetFileExtension(
    _In_ PUNICODE_STRING FileName,
    _Out_writes_(MaxLength) PWCHAR Buffer,
    _In_ ULONG MaxLength
);

#endif /* SENTINELGUARD_DRIVER_H */
