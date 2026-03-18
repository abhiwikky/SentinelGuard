/*
 * SentinelGuard Minifilter Driver - Main Entry
 *
 * Registers with the Filter Manager to observe file system operations
 * and forward structured telemetry events to user mode via a
 * communication port.
 */

#include "driver.h"

/* ─── Globals ─────────────────────────────────────────────────────────── */

SG_DRIVER_DATA g_DriverData = { 0 };

/* ─── Forward Declarations ────────────────────────────────────────────── */

DRIVER_INITIALIZE DriverEntry;
NTSTATUS
SgFilterUnload(
    _In_ FLT_FILTER_UNLOAD_FLAGS Flags
);

NTSTATUS
SgInstanceSetup(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_SETUP_FLAGS Flags,
    _In_ DEVICE_TYPE VolumeDeviceType,
    _In_ FLT_FILESYSTEM_TYPE VolumeFilesystemType
);

NTSTATUS
SgInstanceQueryTeardown(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_QUERY_TEARDOWN_FLAGS Flags
);

/* ─── Operation Registration ─────────────────────────────────────────── */

CONST FLT_OPERATION_REGISTRATION g_Callbacks[] = {
    {
        IRP_MJ_CREATE,
        0,
        SgPreCreateCallback,
        NULL
    },
    {
        IRP_MJ_WRITE,
        0,
        SgPreWriteCallback,
        NULL
    },
    {
        IRP_MJ_SET_INFORMATION,
        0,
        SgPreSetInfoCallback,
        NULL
    },
    {
        IRP_MJ_DIRECTORY_CONTROL,
        0,
        SgPreDirCtrlCallback,
        NULL
    },
    { IRP_MJ_OPERATION_END }
};

/* ─── Context Registration (none needed) ──────────────────────────────── */

CONST FLT_CONTEXT_REGISTRATION g_ContextRegistration[] = {
    { FLT_CONTEXT_END }
};

/* ─── Filter Registration ─────────────────────────────────────────────── */

CONST FLT_REGISTRATION g_FilterRegistration = {
    sizeof(FLT_REGISTRATION),          /* Size */
    FLT_REGISTRATION_VERSION,          /* Version */
    0,                                 /* Flags */
    g_ContextRegistration,             /* Context */
    g_Callbacks,                       /* Operation callbacks */
    SgFilterUnload,                    /* FilterUnload */
    SgInstanceSetup,                   /* InstanceSetup */
    SgInstanceQueryTeardown,           /* InstanceQueryTeardown */
    NULL,                              /* InstanceTeardownStart */
    NULL,                              /* InstanceTeardownComplete */
    NULL,                              /* GenerateFileName */
    NULL,                              /* NormalizeNameComponent */
    NULL                               /* NormalizeContextCleanup */
#if FLT_MGR_LONGHORN
    , NULL                             /* TransactionNotification */
    , NULL                             /* NormalizeNameComponentEx */
#endif
};

/* ─── DriverEntry ─────────────────────────────────────────────────────── */

NTSTATUS
DriverEntry(
    _In_ PDRIVER_OBJECT DriverObject,
    _In_ PUNICODE_STRING RegistryPath
)
{
    NTSTATUS status;

    UNREFERENCED_PARAMETER(RegistryPath);

    KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
        "SentinelGuard: DriverEntry called\n"));

    RtlZeroMemory(&g_DriverData, sizeof(g_DriverData));

    /* Register the minifilter with the Filter Manager */
    status = FltRegisterFilter(
        DriverObject,
        &g_FilterRegistration,
        &g_DriverData.FilterHandle
    );

    if (!NT_SUCCESS(status)) {
        KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL,
            "SentinelGuard: FltRegisterFilter failed 0x%08x\n", status));
        return status;
    }

    /* Create the communication port for user-mode agent */
    status = SgCreateCommunicationPort(g_DriverData.FilterHandle);
    if (!NT_SUCCESS(status)) {
        KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL,
            "SentinelGuard: SgCreateCommunicationPort failed 0x%08x\n", status));
        FltUnregisterFilter(g_DriverData.FilterHandle);
        return status;
    }

    /* Start filtering */
    status = FltStartFiltering(g_DriverData.FilterHandle);
    if (!NT_SUCCESS(status)) {
        KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_ERROR_LEVEL,
            "SentinelGuard: FltStartFiltering failed 0x%08x\n", status));
        SgCloseCommunicationPort();
        FltUnregisterFilter(g_DriverData.FilterHandle);
        return status;
    }

    KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
        "SentinelGuard: Driver loaded successfully\n"));

    return STATUS_SUCCESS;
}

/* ─── Filter Unload ───────────────────────────────────────────────────── */

NTSTATUS
SgFilterUnload(
    _In_ FLT_FILTER_UNLOAD_FLAGS Flags
)
{
    UNREFERENCED_PARAMETER(Flags);

    KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
        "SentinelGuard: Unloading driver\n"));

    SgCloseCommunicationPort();

    if (g_DriverData.FilterHandle != NULL) {
        FltUnregisterFilter(g_DriverData.FilterHandle);
        g_DriverData.FilterHandle = NULL;
    }

    KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
        "SentinelGuard: Driver unloaded\n"));

    return STATUS_SUCCESS;
}

/* ─── Instance Setup ──────────────────────────────────────────────────── */

NTSTATUS
SgInstanceSetup(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_SETUP_FLAGS Flags,
    _In_ DEVICE_TYPE VolumeDeviceType,
    _In_ FLT_FILESYSTEM_TYPE VolumeFilesystemType
)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(Flags);

    /* Only attach to disk-based NTFS and ReFS volumes */
    if (VolumeDeviceType != FILE_DEVICE_DISK_FILE_SYSTEM) {
        return STATUS_FLT_DO_NOT_ATTACH;
    }

    if (VolumeFilesystemType != FLT_FSTYPE_NTFS &&
        VolumeFilesystemType != FLT_FSTYPE_REFS) {
        return STATUS_FLT_DO_NOT_ATTACH;
    }

    KdPrintEx((DPFLTR_IHVDRIVER_ID, DPFLTR_INFO_LEVEL,
        "SentinelGuard: Attached to volume (fstype=%d)\n",
        VolumeFilesystemType));

    return STATUS_SUCCESS;
}

/* ─── Instance Query Teardown ─────────────────────────────────────────── */

NTSTATUS
SgInstanceQueryTeardown(
    _In_ PCFLT_RELATED_OBJECTS FltObjects,
    _In_ FLT_INSTANCE_QUERY_TEARDOWN_FLAGS Flags
)
{
    UNREFERENCED_PARAMETER(FltObjects);
    UNREFERENCED_PARAMETER(Flags);

    /* Always allow teardown */
    return STATUS_SUCCESS;
}
