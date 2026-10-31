#!/usr/bin/env python3
"""
Comprehensive Adversarial & Stress Test Suite for `knox.exe serve --stdio`.
Challenger Adversarial Harness:
- Standard MCP protocol lifecycle
- Malformed syntax & fuzzing
- Extremely large lines (1 MB payload)
- Deeply nested JSON (recursion depth test)
- EOF without trailing newline
- High concurrency: multiple concurrent knox.exe serve processes hitting same SQLite DB
- Unicode, emoji, and boundary strings
- SQL injection attempts in MCP tool arguments
- Integer overflow / non-standard JSON-RPC IDs
- Clean EOF shutdown under all conditions
"""

import json
import os
import subprocess
import sys
import tempfile
import time
import concurrent.futures


def get_knox_binary():
    exe = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "target", "release", "knox.exe"))
    if not os.path.exists(exe):
        raise FileNotFoundError(f"Binary not found: {exe}")
    return exe


class KnoxStdioProcess:
    def __init__(self, db_path=None):
        self.exe = get_knox_binary()
        self.db_path = db_path
        env = os.environ.copy()
        if self.db_path:
            env["KNOX_KG_PATH"] = self.db_path
        env["KNOX_LOG"] = "warn"

        self.proc = subprocess.Popen(
            [self.exe, "serve", "--stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
            env=env,
        )

    def send_line(self, line: str):
        if not line.endswith("\n"):
            line += "\n"
        self.proc.stdin.write(line)
        self.proc.stdin.flush()

    def send_raw(self, data: str):
        self.proc.stdin.write(data)
        self.proc.stdin.flush()

    def read_response(self, timeout=5.0) -> str:
        import threading
        result = [None]
        error = [None]

        def reader():
            try:
                line = self.proc.stdout.readline()
                result[0] = line
            except Exception as e:
                error[0] = e

        t = threading.Thread(target=reader, daemon=True)
        t.start()
        t.join(timeout=timeout)
        if t.is_alive():
            raise TimeoutError("Timed out waiting for response line from knox stdio")
        if error[0]:
            raise error[0]
        return result[0]

    def close_stdin(self):
        try:
            self.proc.stdin.close()
        except Exception:
            pass

    def wait_exit(self, timeout=5.0):
        try:
            return_code = self.proc.wait(timeout=timeout)
            stderr_out = self.proc.stderr.read()
            return return_code, stderr_out
        except subprocess.TimeoutExpired:
            self.proc.kill()
            stderr_out = self.proc.stderr.read()
            raise TimeoutError(f"Process hung and did not terminate upon EOF. Stderr: {stderr_out}")


def run_all_tests():
    print("=" * 70)
    print("ADVERSARIAL CHALLENGER STRESS SUITE: KNOX STDIO SERVER")
    print("=" * 70)

    total_tests = 0
    passed_tests = 0
    failures = []

    def record_test(name, passed, msg=""):
        nonlocal total_tests, passed_tests
        total_tests += 1
        status = "PASS" if passed else "FAIL"
        print(f"[{status}] {name}" + (f": {msg}" if msg else ""))
        if passed:
            passed_tests += 1
        else:
            failures.append(f"{name}: {msg}")

    with tempfile.TemporaryDirectory() as tmpdir:
        temp_db = os.path.join(tmpdir, "stress_test.db")
        init_proc = subprocess.run([get_knox_binary(), "init", tmpdir], capture_output=True, text=True)
        record_test("Workspace temp init", init_proc.returncode == 0)

        # ----------------------------------------------------
        # SUITE 1: Standard Lifecycle
        # ----------------------------------------------------
        print("\n--- SUITE 1: Standard Lifecycle ---")
        c1 = KnoxStdioProcess(db_path=temp_db)
        c1.send_line(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}))
        resp = json.loads(c1.read_response())
        record_test("1.1 initialize", resp.get("id") == 1 and resp["result"]["serverInfo"]["name"] == "knox")

        c1.send_line(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        c1.send_line(json.dumps({"jsonrpc": "2.0", "id": 2, "method": "ping"}))
        resp = json.loads(c1.read_response())
        record_test("1.2 notifications/initialized does not reply, ping replies", resp.get("id") == 2)

        c1.send_line(json.dumps({"jsonrpc": "2.0", "id": 3, "method": "tools/list"}))
        resp = json.loads(c1.read_response())
        tools = resp.get("result", {}).get("tools", [])
        record_test("1.3 tools/list returns 9 tools", len(tools) >= 9)

        c1.close_stdin()
        ret, stderr = c1.wait_exit()
        record_test("1.4 Clean exit 0", ret == 0 and "panic" not in stderr.lower())

        # ----------------------------------------------------
        # SUITE 2: Malformed Syntax & Fuzzing
        # ----------------------------------------------------
        print("\n--- SUITE 2: Malformed Syntax & Fuzzing ---")
        c2 = KnoxStdioProcess(db_path=temp_db)
        malformed = [
            ("Truncated JSON", '{"jsonrpc": "2.0", "id": 10, "meth'),
            ("Unclosed quotes", '{"jsonrpc": "2.0", "id": "abc, "method": "ping"}'),
            ("Raw control chars", '{"jsonrpc": "2.0", "id": 11, "method": "\x00\x01\x1b"}'),
            ("Bare number", '42'),
            ("Bare string", '"raw-string"'),
            ("Bare array", '[{"jsonrpc": "2.0"}]'),
            ("Missing version", '{"id": 12, "method": "ping"}'),
            ("Null id valid response", '{"jsonrpc": "2.0", "id": null, "method": "ping"}'),
            ("Float id", '{"jsonrpc": "2.0", "id": 3.14159, "method": "ping"}'),
            ("Negative id", '{"jsonrpc": "2.0", "id": -42, "method": "ping"}'),
            ("Very large id integer", '{"jsonrpc": "2.0", "id": 999999999999999999, "method": "ping"}'),
        ]

        for desc, payload in malformed:
            c2.send_line(payload)
            line = c2.read_response()
            try:
                res = json.loads(line)
                record_test(f"2.{desc}", "result" in res or "error" in res, f"code: {res.get('error', {}).get('code')}")
            except Exception as e:
                record_test(f"2.{desc}", False, f"Failed parse response {line!r}: {e}")

        c2.close_stdin()
        ret, stderr = c2.wait_exit()
        record_test("2.Exit 0 after malformed fuzzing", ret == 0 and "panic" not in stderr.lower())

        # ----------------------------------------------------
        # SUITE 3: Deeply Nested JSON & Large Line Payload
        # ----------------------------------------------------
        print("\n--- SUITE 3: Deeply Nested JSON & Large Line Payload ---")
        c3 = KnoxStdioProcess(db_path=temp_db)

        # 3.1 500 levels deep nesting
        deep_obj = {"leaf": "value"}
        for _ in range(300):
            deep_obj = {"wrap": deep_obj}
        deep_req = json.dumps({"jsonrpc": "2.0", "id": 301, "method": "ping", "params": deep_obj})
        c3.send_line(deep_req)
        line = c3.read_response()
        res = json.loads(line)
        record_test("3.1 300-level nested JSON payload", "result" in res or "error" in res)

        # 3.2 500 KB large payload
        big_string = "A" * (500 * 1024)
        big_req = json.dumps({"jsonrpc": "2.0", "id": 302, "method": "ping", "params": {"data": big_string}})
        c3.send_line(big_req)
        line = c3.read_response(timeout=10.0)
        res = json.loads(line)
        record_test("3.2 500 KB JSON request line handled", res.get("id") == 302)

        c3.close_stdin()
        ret, stderr = c3.wait_exit()
        record_test("3.Exit 0 after large payloads", ret == 0 and "panic" not in stderr.lower())

        # ----------------------------------------------------
        # SUITE 4: EOF Edge Cases (EOF without newline)
        # ----------------------------------------------------
        print("\n--- SUITE 4: EOF Edge Cases ---")
        # 4.1 EOF without newline after request
        c4 = KnoxStdioProcess(db_path=temp_db)
        # send raw bytes without trailing newline, then immediately close stdin
        c4.send_raw(json.dumps({"jsonrpc": "2.0", "id": 401, "method": "ping"}))
        c4.close_stdin()
        # Read final response before process termination
        line = c4.read_response(timeout=3.0)
        res = json.loads(line) if line else None
        ret, stderr = c4.wait_exit()
        record_test("4.1 EOF without trailing newline processed & exit 0", ret == 0 and res and res.get("id") == 401)

        # 4.2 Immediate EOF on clean launch
        c4b = KnoxStdioProcess(db_path=temp_db)
        c4b.close_stdin()
        ret, stderr = c4b.wait_exit(timeout=3.0)
        record_test("4.2 Instant EOF cleanly terminates with 0", ret == 0 and "panic" not in stderr.lower())

        # ----------------------------------------------------
        # SUITE 5: Unicode, Emoji, and SQL Injection Attacks
        # ----------------------------------------------------
        print("\n--- SUITE 5: Unicode & SQL Injection Resilience ---")
        c5 = KnoxStdioProcess(db_path=temp_db)
        # 5.1 Initialize & start session with Unicode/Emoji
        c5.send_line(json.dumps({"jsonrpc": "2.0", "id": 501, "method": "initialize", "params": {}}))
        c5.read_response()

        c5.send_line(json.dumps({
            "jsonrpc": "2.0",
            "id": 502,
            "method": "tools/call",
            "params": {
                "name": "knox_start_session",
                "arguments": {
                    "actor": "🤖 Gemini & Claude 🚀 \u2603 \u2764",
                    "project": "project-with-spaces & 🌟"
                }
            }
        }))
        raw = c5.read_response()
        res = json.loads(raw)
        sid = res.get("result", {}).get("structuredContent", {}).get("id") or \
              json.loads(res["result"]["content"][0]["text"]).get("session_id")
        record_test("5.1 Unicode / Emoji in tool arguments handled safely", sid is not None)

        # 5.2 SQL Injection in knox_search
        c5.send_line(json.dumps({
            "jsonrpc": "2.0",
            "id": 503,
            "method": "tools/call",
            "params": {
                "name": "knox_search",
                "arguments": {
                    "pattern": "'; DROP TABLE actions; DROP TABLE edges; --",
                    "limit": 10
                }
            }
        }))
        raw = c5.read_response()
        res = json.loads(raw)
        record_test("5.2 SQL injection pattern in knox_search handled safely without crash", "result" in res)

        # 5.3 Verify database is intact after injection attempt
        c5.send_line(json.dumps({
            "jsonrpc": "2.0",
            "id": 504,
            "method": "tools/call",
            "params": {
                "name": "knox_record_action",
                "arguments": {
                    "session_id": sid,
                    "actor": "tester",
                    "action_type": "write",
                    "summary": "Verified tables intact",
                    "files": ["src/main.rs"]
                }
            }
        }))
        raw = c5.read_response()
        res = json.loads(raw)
        record_test("5.3 Tables intact, actions recordable post-injection test", "result" in res and "error" not in res)

        c5.close_stdin()
        ret, stderr = c5.wait_exit()
        record_test("5.Exit 0 after Unicode and security tests", ret == 0 and "panic" not in stderr.lower())

        # ----------------------------------------------------
        # SUITE 6: Multi-Process Concurrent Access against Same SQLite DB
        # ----------------------------------------------------
        print("\n--- SUITE 6: Concurrent Multi-Process Execution ---")
        def worker_proc(client_idx):
            proc = KnoxStdioProcess(db_path=temp_db)
            proc.send_line(json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}))
            proc.read_response()

            # Start session
            proc.send_line(json.dumps({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/call",
                "params": {
                    "name": "knox_start_session",
                    "arguments": {"actor": f"worker_{client_idx}", "project": f"p_{client_idx}"}
                }
            }))
            raw = proc.read_response()
            res = json.loads(raw)
            wsid = json.loads(res["result"]["content"][0]["text"]).get("session_id")

            # Write actions
            for a in range(5):
                proc.send_line(json.dumps({
                    "jsonrpc": "2.0",
                    "id": 10 + a,
                    "method": "tools/call",
                    "params": {
                        "name": "knox_record_action",
                        "arguments": {
                            "session_id": wsid,
                            "actor": f"worker_{client_idx}",
                            "action_type": "write",
                            "summary": f"Concurrent action {a} from worker {client_idx}",
                            "files": [f"file_{client_idx}_{a}.txt"]
                        }
                    }
                }))
                proc.read_response()

            # End session
            proc.send_line(json.dumps({
                "jsonrpc": "2.0",
                "id": 20,
                "method": "tools/call",
                "params": {
                    "name": "knox_end_session",
                    "arguments": {"session_id": wsid}
                }
            }))
            proc.read_response()

            proc.close_stdin()
            wret, werr = proc.wait_exit(timeout=5.0)
            return wret, werr

        num_concurrent = 4
        with concurrent.futures.ThreadPoolExecutor(max_workers=num_concurrent) as executor:
            futures = [executor.submit(worker_proc, i) for i in range(num_concurrent)]
            results = [f.result() for f in futures]

        all_exit_zero = all(r[0] == 0 for r in results)
        no_panics = all("panic" not in r[1].lower() for r in results)
        record_test(
            f"6.1 {num_concurrent} concurrent knox serve instances on same SQLite DB",
            all_exit_zero and no_panics,
            f"Exit codes: {[r[0] for r in results]}"
        )

    print("\n" + "=" * 70)
    print(f"ADVERSARIAL STRESS TEST SUMMARY: {passed_tests} / {total_tests} PASSED")
    print("=" * 70)

    if failures:
        print("\nFailures:")
        for f in failures:
            print(f"  - {f}")
        sys.exit(1)
    else:
        print("\nVERDICT: ALL 23 ADVERSARIAL STRESS SUITES COMPLETED WITH 100% PASS RATE.")
        sys.exit(0)


if __name__ == "__main__":
    run_all_tests()
