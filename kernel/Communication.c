//
// Communication implementation
//

#include "SentinelGuard.h"
#include "Communication.h"
#include <ntstrsafe.h>

NTSTATUS CreateCommunicationPort(VOID)
{
    NTSTATUS status;
    UNICODE_STRING portName;
    PSECURITY_DESCRIPTOR sd = NULL;
    OBJECT_ATTRIBUTES oa;

    RtlInitUnicodeString(&portName, SENTINELGUARD_PORT_NAME);

    // Create security descriptor (allow all for now - should be restricted in production)
    status = FltBuildDefaultSecurityDescriptor(&sd, FLT_PORT_ALL_ACCESS);
    if (!NT_SUCCESS(status)) {
        return status;
    }

    InitializeObjectAttributes(
        &oa,
        &portName,
        OBJ_CASE_INSENSITIVE | OBJ_KERNEL_HANDLE,
        NULL,
        sd
    );

    status = FltCreateCommunicationPort(
        g_DriverContext.FilterHandle,
        &g_DriverContext.ServerPort,
        &oa,
        NULL,
        PortConnectNotifyCallback,
        PortDisconnectNotifyCallback,
        PortMessageNotifyCallback,
        1
    );

    if (sd) {
        FltFreeSecurityDescriptor(sd);
    }

    return status;
}

VOID CloseCommunicationPort(VOID)
{
    if (g_DriverContext.ServerPort) {
        FltCloseCommunicationPort(g_DriverContext.ServerPort);
        g_DriverContext.ServerPort = NULL;
    }
}

NTSTATUS SendEventToUserMode(_In_ PFILE_EVENT Event)
{
    if (!g_DriverContext.ClientPort) {
        return STATUS_PORT_DISCONNECTED;
    }

    NTSTATUS status;
    ULONG bytesReturned;

    status = FltSendMessage(
        g_DriverContext.FilterHandle,
        &g_DriverContext.ClientPort,
        Event,
        sizeof(FILE_EVENT),
        NULL,
        &bytesReturned,
        NULL
    );

    return status;
}

NTSTATUS PortConnectNotifyCallback(
    _In_ PFLT_PORT ClientPort,
    _In_opt_ PVOID ServerPortCookie,
    _In_reads_bytes_opt_(SizeOfContext) PVOID ConnectionContext,
    _In_ ULONG SizeOfContext,
    _Flt_ConnectionCookie_Outptr_ PVOID *ConnectionCookie
)
{
    UNREFERENCED_PARAMETER(ServerPortCookie);
    UNREFERENCED_PARAMETER(ConnectionContext);
    UNREFERENCED_PARAMETER(SizeOfContext);

    g_DriverContext.ClientPort = ClientPort;
    *ConnectionCookie = ClientPort;

    return STATUS_SUCCESS;
}

VOID PortDisconnectNotifyCallback(
    _In_opt_ PVOID ConnectionCookie
)
{
    UNREFERENCED_PARAMETER(ConnectionCookie);
    g_DriverContext.ClientPort = NULL;
}

NTSTATUS PortMessageNotifyCallback(
    _In_opt_ PVOID PortCookie,
    _In_reads_bytes_opt_(InputBufferSize) PVOID InputBuffer,
    _In_ ULONG InputBufferSize,
    _Out_writes_bytes_to_opt_(OutputBufferSize, *ReturnOutputBufferLength) PVOID OutputBuffer,
    _In_ ULONG OutputBufferSize,
    _Flt_ReturnOutputBufferLength_Out_ PULONG ReturnOutputBufferLength
)
{
    UNREFERENCED_PARAMETER(PortCookie);
    UNREFERENCED_PARAMETER(InputBuffer);
    UNREFERENCED_PARAMETER(InputBufferSize);
    UNREFERENCED_PARAMETER(OutputBuffer);
    UNREFERENCED_PARAMETER(OutputBufferSize);

    *ReturnOutputBufferLength = 0;
    return STATUS_SUCCESS;
}

