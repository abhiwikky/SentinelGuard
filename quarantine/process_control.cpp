//
// NT API implementation
//

#include "process_control.h"
#include <winternl.h>

PNtSuspendProcess NtSuspendProcess = NULL;
PNtResumeProcess NtResumeProcess = NULL;

bool InitializeProcessControl() {
    HMODULE ntdll = GetModuleHandleW(L"ntdll.dll");
    if (!ntdll) {
        return false;
    }

    NtSuspendProcess = (PNtSuspendProcess)GetProcAddress(ntdll, "NtSuspendProcess");
    NtResumeProcess = (PNtResumeProcess)GetProcAddress(ntdll, "NtResumeProcess");

    return (NtSuspendProcess != NULL && NtResumeProcess != NULL);
}

// Initialize on module load
static bool g_initialized = InitializeProcessControl();

