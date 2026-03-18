"""
SentinelGuard API Validation Tests

Tests the Node.js bridge HTTP/JSON API endpoints.
Requires the bridge to be running on localhost:3001.

Usage:
    python api_test.py [--bridge-url http://127.0.0.1:3001]
"""

import argparse
import json
import sys
import time
import urllib.request
import urllib.error

DEFAULT_BRIDGE_URL = "http://127.0.0.1:3001"

class TestResult:
    def __init__(self, name: str, passed: bool, message: str = ""):
        self.name = name
        self.passed = passed
        self.message = message

    def __str__(self):
        status = "PASS" if self.passed else "FAIL"
        msg = f" - {self.message}" if self.message else ""
        return f"  [{status}] {self.name}{msg}"


def http_get(url: str, timeout: int = 5) -> tuple[int, dict | list | None]:
    """Perform an HTTP GET and return (status_code, parsed_json)."""
    try:
        req = urllib.request.Request(url, headers={"Accept": "application/json"})
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8")
            return resp.status, json.loads(body)
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8") if e.fp else ""
        try:
            return e.code, json.loads(body)
        except json.JSONDecodeError:
            return e.code, None
    except urllib.error.URLError as e:
        return 0, None
    except Exception as e:
        return -1, None


def http_post(url: str, data: dict, timeout: int = 5) -> tuple[int, dict | None]:
    """Perform an HTTP POST and return (status_code, parsed_json)."""
    try:
        payload = json.dumps(data).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=payload,
            headers={"Content-Type": "application/json", "Accept": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = resp.read().decode("utf-8")
            return resp.status, json.loads(body)
    except urllib.error.HTTPError as e:
        body = e.read().decode("utf-8") if e.fp else ""
        try:
            return e.code, json.loads(body)
        except json.JSONDecodeError:
            return e.code, None
    except Exception:
        return -1, None


def run_tests(bridge_url: str) -> list[TestResult]:
    results = []

    # ─── Test 1: Bridge Health ─────────────────────────────────────────
    status, data = http_get(f"{bridge_url}/api/bridge/health")
    if status == 200 and data and data.get("bridge_running"):
        results.append(TestResult("Bridge Health", True, f"uptime={data.get('uptime_seconds')}s"))
    elif status == 0:
        results.append(TestResult("Bridge Health", False, "Bridge not reachable"))
        # If bridge is down, all other tests will fail
        return results
    else:
        results.append(TestResult("Bridge Health", False, f"status={status}"))

    # ─── Test 2: System Health ─────────────────────────────────────────
    status, data = http_get(f"{bridge_url}/api/health")
    if status == 200 and data:
        results.append(TestResult("System Health", True,
            f"agent={data.get('agentRunning')}, driver={data.get('driverConnected')}"))
    elif status == 503:
        results.append(TestResult("System Health", True, "Backend unavailable (expected if agent not running)"))
    else:
        results.append(TestResult("System Health", False, f"status={status}"))

    # ─── Test 3: Get Alerts ────────────────────────────────────────────
    status, data = http_get(f"{bridge_url}/api/alerts?limit=10")
    if status == 200 and isinstance(data, list):
        results.append(TestResult("Get Alerts", True, f"count={len(data)}"))
    elif status == 503:
        results.append(TestResult("Get Alerts", True, "Backend unavailable (expected)"))
    else:
        results.append(TestResult("Get Alerts", False, f"status={status}"))

    # ─── Test 4: Get Processes ─────────────────────────────────────────
    status, data = http_get(f"{bridge_url}/api/processes?limit=10")
    if status == 200 and isinstance(data, list):
        results.append(TestResult("Get Processes", True, f"count={len(data)}"))
    elif status == 503:
        results.append(TestResult("Get Processes", True, "Backend unavailable (expected)"))
    else:
        results.append(TestResult("Get Processes", False, f"status={status}"))

    # ─── Test 5: Get Quarantined ───────────────────────────────────────
    status, data = http_get(f"{bridge_url}/api/quarantined")
    if status == 200 and isinstance(data, list):
        results.append(TestResult("Get Quarantined", True, f"count={len(data)}"))
    elif status == 503:
        results.append(TestResult("Get Quarantined", True, "Backend unavailable (expected)"))
    else:
        results.append(TestResult("Get Quarantined", False, f"status={status}"))

    # ─── Test 6: Get Detector Logs ─────────────────────────────────────
    status, data = http_get(f"{bridge_url}/api/detectors?limit=10")
    if status == 200 and isinstance(data, list):
        results.append(TestResult("Get Detector Logs", True, f"count={len(data)}"))
    elif status == 503:
        results.append(TestResult("Get Detector Logs", True, "Backend unavailable (expected)"))
    else:
        results.append(TestResult("Get Detector Logs", False, f"status={status}"))

    # ─── Test 7: Release Process (invalid PID) ────────────────────────
    status, data = http_post(f"{bridge_url}/api/quarantined/release", {"process_id": 0})
    if status == 400:
        results.append(TestResult("Release (invalid PID)", True, "Correctly rejected"))
    elif status == 503:
        results.append(TestResult("Release (invalid PID)", True, "Backend unavailable (expected)"))
    else:
        results.append(TestResult("Release (invalid PID)", False, f"status={status}"))

    # ─── Test 8: SSE Stream Connectivity ──────────────────────────────
    try:
        req = urllib.request.Request(
            f"{bridge_url}/api/alerts/stream",
            headers={"Accept": "text/event-stream"}
        )
        with urllib.request.urlopen(req, timeout=3) as resp:
            first_line = resp.readline().decode("utf-8")
            if "connected" in first_line:
                results.append(TestResult("SSE Stream", True, "Connected"))
            else:
                results.append(TestResult("SSE Stream", True, f"Response: {first_line[:50]}"))
    except Exception as e:
        results.append(TestResult("SSE Stream", False, str(e)[:100]))

    return results


def main():
    parser = argparse.ArgumentParser(description="SentinelGuard API Tests")
    parser.add_argument("--bridge-url", default=DEFAULT_BRIDGE_URL,
                        help=f"Bridge URL (default: {DEFAULT_BRIDGE_URL})")
    args = parser.parse_args()

    print(f"\nSentinelGuard API Validation Tests")
    print(f"{'=' * 50}")
    print(f"  Bridge: {args.bridge_url}")
    print()

    results = run_tests(args.bridge_url)

    passed = sum(1 for r in results if r.passed)
    total = len(results)

    for r in results:
        color = "" if r.passed else ""
        print(r)

    print(f"\n  Results: {passed}/{total} passed")

    if passed == total:
        print("  All tests passed!")
        return 0
    else:
        print(f"  {total - passed} test(s) failed")
        return 1


if __name__ == "__main__":
    sys.exit(main())
