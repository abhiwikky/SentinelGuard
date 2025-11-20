//
// Quarantine Module
// Process suspension and isolation
//

#pragma once

#include <windows.h>
#include <string>

class QuarantineModule {
public:
    QuarantineModule();
    ~QuarantineModule();

    bool SuspendProcess(DWORD processId);
    bool KillProcess(DWORD processId);
    bool BlockFileHandles(DWORD processId);
    bool IsolateFiles(DWORD processId, const std::wstring& quarantinePath);
    bool SetFileACLs(const std::wstring& filePath, bool readOnly);

private:
    HANDLE OpenProcessWithAccess(DWORD processId, DWORD desiredAccess);
    bool IsAdmin();
};

