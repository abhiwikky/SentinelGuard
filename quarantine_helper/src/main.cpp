/*
 * SentinelGuard Quarantine Helper
 *
 * Standalone CLI tool for suspending and resuming processes.
 * Uses NtSuspendProcess / NtResumeProcess from ntdll.dll.
 *
 * Usage:
 *   quarantine_helper.exe --suspend <PID>
 *   quarantine_helper.exe --release <PID>
 *
 * Exit codes:
 *   0 = success
 *   1 = invalid arguments
 *   2 = process not found / already exited
 *   3 = access denied
 *   4 = process already in target state
 *   5 = internal error
 */

#include <windows.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* NtSuspendProcess / NtResumeProcess are exported from ntdll.dll.
 * They are documented in the Windows Driver internals but not in the
 * official SDK headers, so we load them dynamically. */

typedef LONG NTSTATUS;
#define STATUS_SUCCESS ((NTSTATUS)0x00000000L)

typedef NTSTATUS(NTAPI *PFN_NtSuspendProcess)(HANDLE ProcessHandle);
typedef NTSTATUS(NTAPI *PFN_NtResumeProcess)(HANDLE ProcessHandle);

static PFN_NtSuspendProcess pfnNtSuspendProcess = NULL;
static PFN_NtResumeProcess pfnNtResumeProcess = NULL;

/* Load ntdll functions */
static int LoadNtdllFunctions(void)
{
    HMODULE hNtdll = GetModuleHandleW(L"ntdll.dll");
    if (!hNtdll)
    {
        fprintf(stderr, "ERROR: Failed to get ntdll.dll handle\n");
        return 0;
    }

    pfnNtSuspendProcess = (PFN_NtSuspendProcess)GetProcAddress(hNtdll, "NtSuspendProcess");
    pfnNtResumeProcess = (PFN_NtResumeProcess)GetProcAddress(hNtdll, "NtResumeProcess");

    if (!pfnNtSuspendProcess || !pfnNtResumeProcess)
    {
        fprintf(stderr, "ERROR: Failed to resolve NtSuspendProcess/NtResumeProcess\n");
        return 0;
    }

    return 1;
}

/* Open a process with required access rights */
static HANDLE OpenTargetProcess(DWORD processId, DWORD *errorCode)
{
    HANDLE hProcess = OpenProcess(
        PROCESS_SUSPEND_RESUME | PROCESS_QUERY_LIMITED_INFORMATION,
        FALSE,
        processId);

    if (!hProcess)
    {
        *errorCode = GetLastError();
        return NULL;
    }

    /* Verify the process is still alive */
    DWORD exitCode;
    if (GetExitCodeProcess(hProcess, &exitCode))
    {
        if (exitCode != STILL_ACTIVE)
        {
            CloseHandle(hProcess);
            *errorCode = ERROR_PROCESS_ABORTED;
            return NULL;
        }
    }

    *errorCode = 0;
    return hProcess;
}

static void PrintUsage(const char *progName)
{
    fprintf(stderr, "Usage:\n");
    fprintf(stderr, "  %s --suspend <PID>\n", progName);
    fprintf(stderr, "  %s --release <PID>\n", progName);
}

int main(int argc, char *argv[])
{
    if (argc != 3)
    {
        PrintUsage(argv[0]);
        return 1;
    }

    const char *action = argv[1];
    DWORD processId = (DWORD)strtoul(argv[2], NULL, 10);

    if (processId == 0)
    {
        fprintf(stderr, "ERROR: Invalid PID: %s\n", argv[2]);
        return 1;
    }

    int isSuspend;
    if (strcmp(action, "--suspend") == 0)
    {
        isSuspend = 1;
    }
    else if (strcmp(action, "--release") == 0)
    {
        isSuspend = 0;
    }
    else
    {
        fprintf(stderr, "ERROR: Unknown action: %s\n", action);
        PrintUsage(argv[0]);
        return 1;
    }

    /* Load ntdll functions */
    if (!LoadNtdllFunctions())
    {
        return 5;
    }

    /* Open the target process */
    DWORD openError;
    HANDLE hProcess = OpenTargetProcess(processId, &openError);
    if (!hProcess)
    {
        if (openError == ERROR_INVALID_PARAMETER ||
            openError == ERROR_PROCESS_ABORTED)
        {
            fprintf(stderr, "ERROR: Process %lu not found or already exited\n", processId);
            return 2;
        }
        else if (openError == ERROR_ACCESS_DENIED)
        {
            fprintf(stderr, "ERROR: Access denied for process %lu. Run as Administrator.\n", processId);
            return 3;
        }
        else
        {
            fprintf(stderr, "ERROR: Failed to open process %lu (error %lu)\n", processId, openError);
            return 5;
        }
    }

    /* Perform the action */
    NTSTATUS status;
    if (isSuspend)
    {
        status = pfnNtSuspendProcess(hProcess);
        if (status == STATUS_SUCCESS)
        {
            printf("OK: Process %lu suspended\n", processId);
        }
        else
        {
            fprintf(stderr, "ERROR: NtSuspendProcess failed for PID %lu (NTSTATUS 0x%08X)\n",
                    processId, (unsigned int)status);
            CloseHandle(hProcess);
            return 5;
        }
    }
    else
    {
        status = pfnNtResumeProcess(hProcess);
        if (status == STATUS_SUCCESS)
        {
            printf("OK: Process %lu resumed\n", processId);
        }
        else
        {
            fprintf(stderr, "ERROR: NtResumeProcess failed for PID %lu (NTSTATUS 0x%08X)\n",
                    processId, (unsigned int)status);
            CloseHandle(hProcess);
            return 5;
        }
    }

    CloseHandle(hProcess);
    return 0;
}
