//
// ETW Header
//

#pragma once

#include <fltKernel.h>

NTSTATUS RegisterETWProvider(VOID);
NTSTATUS StartETWTraceSession(VOID);
VOID StopETWTraceSession(VOID);
NTSTATUS MonitorProcessCreation(VOID);
NTSTATUS MonitorRegistryChanges(VOID);
NTSTATUS MonitorShadowCopyDeletion(VOID);

