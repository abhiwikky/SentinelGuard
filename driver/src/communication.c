/*
 * SentinelGuard Minifilter Driver - Communication Port
 *
 * Creates and manages the Filter Manager communication port used to
 * send file-system telemetry events from kernel mode to the user-mode
 * Rust agent.
 */

#include "driver.h"
#include "communication.h"

/* ─── Port Connect Callback ───────────────────────────────────────────── */

NTSTATUS
SgPortConnect(
    _In_ PFLT_PORT ClientPort,
    _In_opt_ PVOID ServerPortCookie,
    _In_reads_bytes_opt_(SizeOfContext) PVOID ConnectionContext,
    _In_ ULONG SizeOfContext,
    _Outptr_result_maybenull_ PVOID *ConnectionPortCookie
)
{
    UNREFERENCED_PARAMETER(ServerPortCookie);
    UNREFERENCED_PARAMETER(ConnectionContext);
    UNREFERENCED_PARAMETER(SizeOfContext);

    /* Only allow one client connection at a time */
    if (g_DriverData.ClientConnected) {
        KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_WARNING_LEVEL,
            "SentinelGuard: Rejecting connection - client already connected\n"));
        *ConnectionPortCookie = NULL;
        return STATUS_ALREADY_REGISTERED;
    }

    g_DriverData.ClientPort = ClientPort;
    g_DriverData.ClientConnected = TRUE;
    *ConnectionPortCookie = NULL;

    KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
        "SentinelGuard: User-mode client connected\n"));

    return STATUS_SUCCESS;
}

/* ─── Port Disconnect Callback ────────────────────────────────────────── */

VOID
SgPortDisconnect(
    _In_opt_ PVOID ConnectionCookie
)
{
    UNREFERENCED_PARAMETER(ConnectionCookie);

    KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
        "SentinelGuard: User-mode client disconnected\n"));

    FltCloseClientPort(g_DriverData.FilterHandle, &g_DriverData.ClientPort);
    g_DriverData.ClientPort = NULL;
    g_DriverData.ClientConnected = FALSE;
}

/* ─── Create Communication Port ───────────────────────────────────────── */

NTSTATUS
SgCreateCommunicationPort(
    _In_ PFLT_FILTER FilterHandle
)
{
    NTSTATUS status;
    UNICODE_STRING portName;
    OBJECT_ATTRIBUTES objAttrs;
    PSECURITY_DESCRIPTOR sd = NULL;

    RtlInitUnicodeString(&portName, SG_PORT_NAME);

    /*
     * Build a security descriptor that allows admin access only.
     * "D:P(A;;GA;;;BA)" = DACL Protected, Allow Generic All to Built-in Admins.
     */
    status = FltBuildDefaultSecurityDescriptor(&sd, FLT_PORT_ALL_ACCESS);
    if (!NT_SUCCESS(status)) {
        KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL,
            "SentinelGuard: FltBuildDefaultSecurityDescriptor failed 0x%08x\n",
            status));
        return status;
    }

    InitializeObjectAttributes(
        &objAttrs,
        &portName,
        OBJ_KERNEL_HANDLE | OBJ_CASE_INSENSITIVE,
        NULL,
        sd
    );

    status = FltCreateCommunicationPort(
        FilterHandle,
        &g_DriverData.ServerPort,
        &objAttrs,
        NULL,                   /* ServerPortCookie */
        SgPortConnect,
        SgPortDisconnect,
        NULL,                   /* MessageNotify - we only send, not receive */
        SG_MAX_CONNECTIONS
    );

    FltFreeSecurityDescriptor(sd);

    if (!NT_SUCCESS(status)) {
        KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL,
            "SentinelGuard: FltCreateCommunicationPort failed 0x%08x\n",
            status));
    } else {
        KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
            "SentinelGuard: Communication port created on %wZ\n", &portName));
    }

    return status;
}

/* ─── Close Communication Port ────────────────────────────────────────── */

VOID
SgCloseCommunicationPort(VOID)
{
    if (g_DriverData.ServerPort != NULL) {
        FltCloseCommunicationPort(g_DriverData.ServerPort);
        g_DriverData.ServerPort = NULL;
    }
}

/* ─── Send Event to User Mode ─────────────────────────────────────────── */

NTSTATUS
SgSendEvent(
    _In_ PSG_EVENT Event
)
{
    NTSTATUS status;
    LARGE_INTEGER timeout;
    ULONG replyLength = 0;

    /* If no client is connected, silently drop the event */
    if (!g_DriverData.ClientConnected || g_DriverData.ClientPort == NULL) {
        return STATUS_PORT_DISCONNECTED;
    }

    /* 5ms timeout – short enough to avoid freezing the filesystem,
     * long enough that events are still delivered during brief agent hiccups.
     * The kernel-side filtering in operations.c eliminates ~90% of noise,
     * so the agent's queue rarely fills up. */
    timeout.QuadPart = -50000LL; /* 5ms in 100ns units, negative = relative */

    status = FltSendMessage(
        g_DriverData.FilterHandle,
        &g_DriverData.ClientPort,
        Event,
        sizeof(SG_EVENT),
        NULL,           /* No reply buffer */
        &replyLength,
        &timeout
    );

    if (!NT_SUCCESS(status) && status != STATUS_TIMEOUT) {
        KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_WARNING_LEVEL,
            "SentinelGuard: FltSendMessage failed 0x%08x\n", status));
    }

    return status;
}
