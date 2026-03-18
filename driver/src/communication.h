#ifndef SENTINELGUARD_COMMUNICATION_H
#define SENTINELGUARD_COMMUNICATION_H

#include "driver.h"

/*
 * Communication module provides the Filter Manager communication port
 * for sending events from kernel to user mode.
 *
 * Architecture:
 *   - One server port created at driver load
 *   - One client connection allowed at a time (the Rust agent)
 *   - Events are sent asynchronously via FltSendMessage
 *   - If no client is connected, events are silently dropped
 */

#endif /* SENTINELGUARD_COMMUNICATION_H */
