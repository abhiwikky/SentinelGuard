//
// Quarantine Module Implementation
//

#include "quarantine.h"
#include "process_control.h"
#include <iostream>
#include <vector>
#include <sstream>

QuarantineModule::QuarantineModule() {
    if (!IsAdmin()) {
        std::wcerr << L"Warning: Not running as administrator. Some operations may fail." << std::endl;
    }
}

QuarantineModule::~QuarantineModule() {
}

bool QuarantineModule::IsAdmin() {
    BOOL isAdmin = FALSE;
    PSID adminGroup = NULL;
    SID_IDENTIFIER_AUTHORITY ntAuthority = SECURITY_NT_AUTHORITY;

    if (AllocateAndInitializeSid(
        &ntAuthority,
        2,
        SECURITY_BUILTIN_DOMAIN_RID,
        DOMAIN_ALIAS_RID_ADMINS,
        0, 0, 0, 0, 0, 0,
        &adminGroup)) {
        CheckTokenMembership(NULL, adminGroup, &isAdmin);
        FreeSid(adminGroup);
    }

    return isAdmin == TRUE;
}

bool QuarantineModule::SuspendProcess(DWORD processId) {
    HANDLE hProcess = OpenProcessWithAccess(processId, PROCESS_SUSPEND_RESUME);
    if (hProcess == NULL) {
        std::wcerr << L"Failed to open process " << processId << std::endl;
        return false;
    }

    // Use NtSuspendProcess via NT API
    NTSTATUS status = NtSuspendProcess(hProcess);
    CloseHandle(hProcess);

    if (status != STATUS_SUCCESS) {
        std::wcerr << L"Failed to suspend process " << processId << std::endl;
        return false;
    }

    std::wcout << L"Process " << processId << L" suspended successfully" << std::endl;
    return true;
}

bool QuarantineModule::KillProcess(DWORD processId) {
    HANDLE hProcess = OpenProcessWithAccess(processId, PROCESS_TERMINATE);
    if (hProcess == NULL) {
        return false;
    }

    BOOL result = TerminateProcess(hProcess, 1);
    CloseHandle(hProcess);

    return result == TRUE;
}

bool QuarantineModule::BlockFileHandles(DWORD processId) {
    // This would require more complex implementation
    // For now, suspending the process effectively blocks file operations
    return SuspendProcess(processId);
}

bool QuarantineModule::IsolateFiles(DWORD processId, const std::wstring& quarantinePath) {
    // Create quarantine directory if it doesn't exist
    CreateDirectoryW(quarantinePath.c_str(), NULL);

    // In production, would enumerate and move files written by the process
    // This is a placeholder
    return true;
}

bool QuarantineModule::SetFileACLs(const std::wstring& filePath, bool readOnly) {
    // Set file to read-only
    DWORD attributes = GetFileAttributesW(filePath.c_str());
    if (attributes == INVALID_FILE_ATTRIBUTES) {
        return false;
    }

    if (readOnly) {
        attributes |= FILE_ATTRIBUTE_READONLY;
    } else {
        attributes &= ~FILE_ATTRIBUTE_READONLY;
    }

    return SetFileAttributesW(filePath.c_str(), attributes) != 0;
}

HANDLE QuarantineModule::OpenProcessWithAccess(DWORD processId, DWORD desiredAccess) {
    HANDLE hProcess = OpenProcess(desiredAccess, FALSE, processId);
    return hProcess;
}

int wmain(int argc, wchar_t* argv[]) {
    if (argc < 3) {
        std::wcout << L"Usage: quarantine.exe --suspend <pid>" << std::endl;
        std::wcout << L"       quarantine.exe --kill <pid>" << std::endl;
        return 1;
    }

    QuarantineModule quarantine;
    std::wstring action = argv[1];
    DWORD processId = _wtoi(argv[2]);

    if (action == L"--suspend") {
        if (quarantine.SuspendProcess(processId)) {
            return 0;
        }
    } else if (action == L"--kill") {
        if (quarantine.KillProcess(processId)) {
            return 0;
        }
    } else {
        std::wcerr << L"Unknown action: " << action << std::endl;
        return 1;
    }

    return 1;
}

