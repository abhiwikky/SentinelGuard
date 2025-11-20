//
// NT API declarations for process control
//

#pragma once

#include <windows.h>

typedef NTSTATUS (WINAPI *PNtSuspendProcess)(HANDLE ProcessHandle);
typedef NTSTATUS (WINAPI *PNtResumeProcess)(HANDLE ProcessHandle);

extern PNtSuspendProcess NtSuspendProcess;
extern PNtResumeProcess NtResumeProcess;

bool InitializeProcessControl();

