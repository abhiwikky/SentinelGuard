//
// Communication with user-mode agent
//

#pragma once

#include "SentinelGuard.h"

NTSTATUS CreateCommunicationPort(VOID);
VOID CloseCommunicationPort(VOID);
NTSTATUS SendEventToUserMode(_In_ PFILE_EVENT Event);

NTSTATUS PortConnectNotifyCallback(
    _In_ PFLT_PORT ClientPort,
    _In_opt_ PVOID ServerPortCookie,
    _In_reads_bytes_opt_(SizeOfContext) PVOID ConnectionContext,
    _In_ ULONG SizeOfContext,
    _Flt_ConnectionCookie_Outptr_ PVOID *ConnectionCookie
);

VOID PortDisconnectNotifyCallback(
    _In_opt_ PVOID ConnectionCookie
);

NTSTATUS PortMessageNotifyCallback(
    _In_opt_ PVOID PortCookie,
    _In_reads_bytes_opt_(InputBufferSize) PVOID InputBuffer,
    _In_ ULONG InputBufferSize,
    _Out_writes_bytes_to_opt_(OutputBufferSize, *ReturnOutputBufferLength) PVOID OutputBuffer,
    _In_ ULONG OutputBufferSize,
    _Flt_ReturnOutputBufferLength_Out_ PULONG ReturnOutputBufferLength
);

