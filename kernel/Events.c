//
// Event processing implementation
//

#include "SentinelGuard.h"
#include "Events.h"
#include "Communication.h"
#include <ntstrsafe.h>

VOID ProcessFileEvent(
    _In_ PFLT_CALLBACK_DATA Data,
    _In_ EVENT_TYPE EventType
)
{
    NTSTATUS status;
    FILE_EVENT event = { 0 };
    PFLT_IO_PARAMETER_BLOCK iopb = Data->Iopb;

    // Get process ID
    event.ProcessId = FltGetRequestorProcessId(Data);

    // Get process path
    status = GetProcessPath(event.ProcessId, event.ProcessPath, sizeof(event.ProcessPath) / sizeof(WCHAR));
    if (!NT_SUCCESS(status)) {
        RtlZeroMemory(event.ProcessPath, sizeof(event.ProcessPath));
    }

    // Get file path
    PFLT_FILE_NAME_INFORMATION nameInfo = NULL;
    status = FltGetFileNameInformation(
        Data,
        FLT_FILE_NAME_NORMALIZED | FLT_FILE_NAME_QUERY_DEFAULT,
        &nameInfo
    );

    if (NT_SUCCESS(status)) {
        status = FltParseFileNameInformation(nameInfo);
        if (NT_SUCCESS(status)) {
            GetFilePath(nameInfo, event.FilePath, sizeof(event.FilePath) / sizeof(WCHAR));
        }
        FltReleaseFileNameInformation(nameInfo);
    }

    // Set event type
    event.Type = EventType;

    // Get timestamp
    LARGE_INTEGER systemTime;
    KeQuerySystemTime(&systemTime);
    event.Timestamp = systemTime.QuadPart;

    // Get bytes read/written
    if (EventType == EventFileWrite || EventType == EventFileRead) {
        event.BytesRead = (EventType == EventFileRead) ? iopb->Parameters.Read.Length : 0;
        event.BytesWritten = (EventType == EventFileWrite) ? iopb->Parameters.Write.Length : 0;

        // Calculate entropy preview for write operations
        if (EventType == EventFileWrite && Data->Iopb->Parameters.Write.Length > 0) {
            // Note: In production, we'd need to read the buffer, but for now we'll skip
            // This requires careful handling of the buffer location
        }
    }

    // Send event to user mode
    SendEventToUserMode(&event);
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

    status = PsLookupProcessByProcessId((HANDLE)(ULONG_PTR)ProcessId, &process);
    if (!NT_SUCCESS(status)) {
        return status;
    }

    status = SeLocateProcessImageName(process, &imagePath);
    if (NT_SUCCESS(status) && imagePath) {
        ULONG copySize = min(BufferSize - 1, imagePath->Length / sizeof(WCHAR));
        RtlCopyMemory(ProcessPath, imagePath->Buffer, copySize * sizeof(WCHAR));
        ProcessPath[copySize] = L'\0';
    }

    if (process) {
        ObDereferenceObject(process);
    }

    return status;
}

NTSTATUS GetFilePath(
    _In_ PFLT_FILE_NAME_INFORMATION FileNameInfo,
    _Out_ PWCHAR FilePath,
    _In_ ULONG BufferSize
)
{
    if (!FileNameInfo || !FileNameInfo->Name.Buffer) {
        return STATUS_INVALID_PARAMETER;
    }

    ULONG copySize = min(BufferSize - 1, FileNameInfo->Name.Length / sizeof(WCHAR));
    RtlCopyMemory(FilePath, FileNameInfo->Name.Buffer, copySize * sizeof(WCHAR));
    FilePath[copySize] = L'\0';

    return STATUS_SUCCESS;
}

UCHAR CalculateEntropy(
    _In_ PUCHAR Data,
    _In_ ULONG Length
)
{
    if (!Data || Length == 0) {
        return 0;
    }

    ULONG frequency[256] = { 0 };
    ULONG i;
    double entropy = 0.0;
    double probability;

    // Count byte frequencies
    for (i = 0; i < Length; i++) {
        frequency[Data[i]]++;
    }

    // Calculate Shannon entropy
    for (i = 0; i < 256; i++) {
        if (frequency[i] > 0) {
            probability = (double)frequency[i] / Length;
            entropy -= probability * log2(probability);
        }
    }

    // Normalize to 0-255 range (entropy max is 8.0 for bytes)
    return (UCHAR)((entropy / 8.0) * 255.0);
}

