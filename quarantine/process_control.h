//
// NT API declarations for process control
//

#pragma once

#include <windows.h>

#ifndef NT_SUCCESS
#define NT_SUCCESS(Status) (((NTSTATUS)(Status)) >= 0)
#endif

typedef NTSTATUS (WINAPI *PNtSuspendProcess)(HANDLE ProcessHandle);
typedef NTSTATUS (WINAPI *PNtResumeProcess)(HANDLE ProcessHandle);

extern PNtSuspendProcess NtSuspendProcess;
extern PNtResumeProcess NtResumeProcess;

bool InitializeProcessControl();

