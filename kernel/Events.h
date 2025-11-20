//
// Event processing functions
//

#pragma once

#include "SentinelGuard.h"

VOID ProcessFileEvent(
    _In_ PFLT_CALLBACK_DATA Data,
    _In_ EVENT_TYPE EventType
);

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

