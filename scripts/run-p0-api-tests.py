#!/usr/bin/env python3
"""Scenariusze API P0 emitenta wykonywane na lokalnym wdrozeniu Docker."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import sys
import time
import unittest
import urllib.error
import urllib.request
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any

TEST_HOLDER = "0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc"
TRANSFER_RECIPIENT = "0x976EA74026E726554dB657fA54763abd0C3a0aa9"


@dataclass
class Response:
    status: int
    body: Any
    raw: str


class Client:
    def __init__(self, base: str, timeout: float) -> None:
        self.base = base.rstrip("/")
        self.timeout = timeout

    def request(self, method: str, path: str, body: dict[str, Any] | None = None) -> Response:
        data = json.dumps(body).encode() if body is not None else None
        headers = {"Accept": "application/json", "Connection": "close"}
        if data is not None:
            headers["Content-Type"] = "application/json"
        request = urllib.request.Request(self.base + path, data=data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as result:
                status, raw = result.status, result.read().decode()
        except urllib.error.HTTPError as error:
            status, raw = error.code, error.read().decode()
        except urllib.error.URLError as error:
            raise AssertionError(f"Brak polaczenia z {self.base}: {error.reason}") from error
        try:
            parsed = json.loads(raw) if raw else None
        except json.JSONDecodeError:
            parsed = None
        return Response(status, parsed, raw)

    def get(self, path: str) -> Response:
        return self.request("GET", path)

    def post(self, path: str, body: dict[str, Any] | None = None) -> Response:
        return self.request("POST", path, body or {})


def require(response: Response, status: int) -> dict[str, Any]:
    if response.status != status or not isinstance(response.body, dict):
        raise AssertionError(f"Oczekiwano HTTP {status} z JSON, otrzymano {response.status}: {response.raw}")
    return response.body


def rpc_call(url: str, method: str, params: list[Any], timeout: float) -> dict[str, Any]:
    payload = json.dumps(
        {"jsonrpc": "2.0", "id": uuid.uuid4().hex, "method": method, "params": params}
    ).encode()
    request = urllib.request.Request(
        url,
        data=payload,
        headers={"Content-Type": "application/json", "Connection": "close"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = json.loads(response.read().decode())
    except (urllib.error.URLError, json.JSONDecodeError) as error:
        raise AssertionError(f"Niepoprawna odpowiedz JSON-RPC z {url}: {error}") from error
    if not isinstance(body, dict):
        raise AssertionError(f"Niepoprawna odpowiedz JSON-RPC: {body!r}")
    return body


def transfer_data(recipient: str, amount_raw: int) -> str:
    address = recipient.removeprefix("0x").lower()
    return "0xa9059cbb" + address.rjust(64, "0") + f"{amount_raw:064x}"


def send_transfer(
    rpc_url: str,
    contract: str,
    sender: str,
    recipient: str,
    amount_raw: int,
    timeout: float,
) -> tuple[bool, dict[str, Any]]:
    response = rpc_call(
        rpc_url,
        "eth_sendTransaction",
        [
            {
                "from": sender,
                "to": contract,
                "data": transfer_data(recipient, amount_raw),
                "gas": "0x493e0",
            }
        ],
        timeout,
    )
    if "error" in response:
        return False, response
    transaction_hash = response.get("result")
    if not isinstance(transaction_hash, str):
        raise AssertionError(f"Brak hasha transakcji transferu: {response!r}")
    receipt = rpc_call(rpc_url, "eth_getTransactionReceipt", [transaction_hash], timeout)
    result = receipt.get("result")
    if not isinstance(result, dict):
        raise AssertionError(f"Brak receipt transferu {transaction_hash}: {receipt!r}")
    return result.get("status") == "0x1", result


def build_tests(issuer: Client, bank: Client, prefix: str) -> type[unittest.TestCase]:
    class IssuerP0ApiTests(unittest.TestCase):
        @classmethod
        def setUpClass(cls) -> None:
            state = require(issuer.get("/api/v1/asset-state"), 200)
            if state.get("state") not in ("active", "warning"):
                raise unittest.SkipTest(
                    "Scenariusze P0 wymagaja stanu active albo warning; "
                    f"aktualny stan: {state.get('state')}"
                )

        def issuance(self, suffix: str, amount_minor: int = 100) -> tuple[str, dict[str, Any]]:
            operation = f"{prefix}-{suffix}"
            payload = {
                "operationId": operation,
                "recipientAddress": TEST_HOLDER,
                "amountUsdMinor": str(amount_minor),
            }
            return operation, require(issuer.post("/api/v1/issuance-orders", payload), 201)

        def confirm_fiat(self, operation: str, amount_minor: int = 100) -> None:
            require(
                bank.post(
                    "/api/v1/reserve-accounts/reserve-rusd/deposits",
                    {
                        "amountMinor": str(amount_minor),
                        "reference": operation,
                        "idempotencyKey": f"issuance-{operation}",
                    },
                ),
                200,
            )

        def test_01_em01_em02_order_waits_for_matching_fiat_and_is_idempotent(self) -> None:
            operation, created = self.issuance("em01-em02")
            replay = require(
                issuer.post(
                    "/api/v1/issuance-orders",
                    {
                        "operationId": operation,
                        "recipientAddress": TEST_HOLDER,
                        "amountUsdMinor": "100",
                    },
                ),
                201,
            )
            self.assertEqual("awaiting_fiat", created["status"])
            self.assertEqual(created["operationId"], replay["operationId"])
            rejected = issuer.post(f"/api/v1/issuance-orders/{operation}/settle")
            self.assertEqual(409, rejected.status, rejected.raw)

            self.confirm_fiat(operation)
            completed = require(issuer.post(f"/api/v1/issuance-orders/{operation}/settle"), 200)
            replayed = require(issuer.post(f"/api/v1/issuance-orders/{operation}/settle"), 200)
            self.assertEqual("completed", completed["status"])
            self.assertEqual(completed.get("transactionHash"), replayed.get("transactionHash"))

        def test_02_em03_concurrent_settlement_mints_once(self) -> None:
            operation, _ = self.issuance("em03")
            self.confirm_fiat(operation)
            path = f"/api/v1/issuance-orders/{operation}/settle"
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
                responses = list(pool.map(lambda _: issuer.post(path), range(2)))
            bodies = [require(response, 200) for response in responses]
            self.assertTrue(all(body["status"] == "completed" for body in bodies))
            self.assertEqual(bodies[0].get("transactionHash"), bodies[1].get("transactionHash"))
            stored = require(issuer.get(f"/api/v1/issuance-orders/{operation}"), 200)
            self.assertEqual(bodies[0].get("transactionHash"), stored.get("transactionHash"))

        def test_03_em04_operation_gate_exposes_an_enforceable_state(self) -> None:
            state = require(issuer.get("/api/v1/asset-state"), 200)
            self.assertIn(state.get("state"), ("active", "warning", "mint_blocked", "data_unavailable", "wind_down"))
            self.assertIn("policyVersion", state)
            self.assertTrue(state.get("reason"))

        def test_04_em05_redemption_burns_and_pays_once_at_parity(self) -> None:
            issued_operation, _ = self.issuance("em05-funding", 200)
            self.confirm_fiat(issued_operation, 200)
            require(issuer.post(f"/api/v1/issuance-orders/{issued_operation}/settle"), 200)

            operation = f"{prefix}-em05-redemption"
            created = require(
                issuer.post(
                    "/api/v1/redemption-orders",
                    {
                        "operationId": operation,
                        "holderAddress": TEST_HOLDER,
                        "tokenAmountRaw": "1000000",
                    },
                ),
                201,
            )
            self.assertEqual("100", created["payoutUsdMinor"])
            path = f"/api/v1/redemption-orders/{operation}/settle"
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
                responses = list(pool.map(lambda _: issuer.post(path), range(2)))
            bodies = [require(response, 200) for response in responses]
            self.assertTrue(all(body["status"] == "completed" for body in bodies))
            self.assertEqual(bodies[0].get("burnTransactionHash"), bodies[1].get("burnTransactionHash"))

    IssuerP0ApiTests.__name__ = "IssuerP0ApiTests"
    IssuerP0ApiTests.__qualname__ = "IssuerP0ApiTests"
    return IssuerP0ApiTests


def build_terminal_wind_down_test(
    issuer: Client,
    bank: Client,
    rpc_url: str,
    timeout: float,
    prefix: str,
) -> type[unittest.TestCase]:
    class IssuerTerminalWindDownApiTest(unittest.TestCase):
        def test_99_em07_wind_down_blocks_mint_and_transfer_but_allows_burn(self) -> None:
            state = require(issuer.get("/api/v1/asset-state"), 200)
            self.assertIn(state.get("state"), ("active", "warning"), state)

            token = require(issuer.get("/api/v1/token"), 200)
            contract = token.get("snapshot", {}).get("contractAddress")
            self.assertIsInstance(contract, str, token)

            funding_operation = f"{prefix}-em07-funding"
            require(
                issuer.post(
                    "/api/v1/issuance-orders",
                    {
                        "operationId": funding_operation,
                        "recipientAddress": TEST_HOLDER,
                        "amountUsdMinor": "300",
                    },
                ),
                201,
            )
            require(
                bank.post(
                    "/api/v1/reserve-accounts/reserve-rusd/deposits",
                    {
                        "amountMinor": "300",
                        "reference": funding_operation,
                        "idempotencyKey": f"issuance-{funding_operation}",
                    },
                ),
                200,
            )
            require(issuer.post(f"/api/v1/issuance-orders/{funding_operation}/settle"), 200)

            transferred, details = send_transfer(
                rpc_url, contract, TEST_HOLDER, TRANSFER_RECIPIENT, 100_000, timeout
            )
            self.assertTrue(transferred, details)

            wind_operation = f"{prefix}-em07-wind-down"
            reason = "Test terminalny uporzadkowanego wygaszania tokenu"
            entered = require(
                issuer.post(
                    "/api/v1/admin/asset-state/wind-down",
                    {"operationId": wind_operation, "reason": reason},
                ),
                200,
            )
            self.assertEqual("wind_down", entered.get("state"), entered)

            replay = require(
                issuer.post(
                    "/api/v1/admin/asset-state/wind-down",
                    {"operationId": wind_operation, "reason": reason},
                ),
                200,
            )
            self.assertEqual("wind_down", replay.get("state"), replay)
            self.assertEqual("wind_down", require(issuer.get("/api/v1/asset-state"), 200).get("state"))

            blocked_operation = f"{prefix}-em07-blocked-mint"
            require(
                issuer.post(
                    "/api/v1/issuance-orders",
                    {
                        "operationId": blocked_operation,
                        "recipientAddress": TEST_HOLDER,
                        "amountUsdMinor": "100",
                    },
                ),
                201,
            )
            require(
                bank.post(
                    "/api/v1/reserve-accounts/reserve-rusd/deposits",
                    {
                        "amountMinor": "100",
                        "reference": blocked_operation,
                        "idempotencyKey": f"issuance-{blocked_operation}",
                    },
                ),
                200,
            )
            blocked = issuer.post(f"/api/v1/issuance-orders/{blocked_operation}/settle")
            blocked_body = require(blocked, 409)
            self.assertEqual("issuance_blocked", blocked_body.get("code"), blocked_body)
            self.assertIn("wind_down", blocked_body.get("error", ""), blocked_body)
            self.assertIn("zwrócona", blocked_body.get("error", ""), blocked_body)
            refund = require(
                bank.get(
                    "/api/v1/reserve-transactions/"
                    f"issuance-refund-{blocked_operation}"
                ),
                200,
            )
            self.assertEqual("withdrawal", refund.get("operationType"), refund)
            self.assertEqual("100", refund.get("amountMinor"), refund)
            self.assertEqual(
                f"refund-to-casp:{blocked_operation}", refund.get("reference"), refund
            )

            transferred, details = send_transfer(
                rpc_url, contract, TEST_HOLDER, TRANSFER_RECIPIENT, 100_000, timeout
            )
            self.assertFalse(transferred, details)

            redemption_operation = f"{prefix}-em07-redemption"
            created = require(
                issuer.post(
                    "/api/v1/redemption-orders",
                    {
                        "operationId": redemption_operation,
                        "holderAddress": TEST_HOLDER,
                        "tokenAmountRaw": "1000000",
                    },
                ),
                201,
            )
            self.assertEqual("100", created.get("payoutUsdMinor"), created)
            redeemed = require(
                issuer.post(f"/api/v1/redemption-orders/{redemption_operation}/settle"), 200
            )
            self.assertEqual("completed", redeemed.get("status"), redeemed)
            self.assertTrue(redeemed.get("burnTransactionHash"), redeemed)

    IssuerTerminalWindDownApiTest.__name__ = "IssuerTerminalWindDownApiTest"
    IssuerTerminalWindDownApiTest.__qualname__ = "IssuerTerminalWindDownApiTest"
    return IssuerTerminalWindDownApiTest


def main() -> int:
    parser = argparse.ArgumentParser(description="Mutujace scenariusze API P0 emitenta.")
    parser.add_argument("--issuer-url", default="http://127.0.0.1:3000")
    parser.add_argument("--bank-url", default="http://127.0.0.1:3100")
    parser.add_argument("--rpc-url", default="http://127.0.0.1:8545")
    parser.add_argument("--timeout", type=float, default=10.0)
    parser.add_argument(
        "--terminal-wind-down",
        action="store_true",
        help="Na koncu nieodwracalnie wlacza wind_down i wykonuje scenariusz EM-07.",
    )
    args = parser.parse_args()
    prefix = "api-p0-issuer-" + uuid.uuid4().hex[:12]
    issuer = Client(args.issuer_url, args.timeout)
    bank = Client(args.bank_url, args.timeout)
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(
        build_tests(issuer, bank, prefix)
    )
    started = time.perf_counter()
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    terminal_result: unittest.TestResult | None = None
    terminal_executed = False
    if args.terminal_wind_down and result.wasSuccessful():
        terminal_executed = True
        terminal_suite = unittest.defaultTestLoader.loadTestsFromTestCase(
            build_terminal_wind_down_test(
                issuer, bank, args.rpc_url, args.timeout, prefix
            )
        )
        terminal_result = unittest.TextTestRunner(verbosity=2).run(terminal_suite)
    elif args.terminal_wind_down:
        print("\nEM-07 pominiety: test terminalny nie zostanie uruchomiony po bledzie bazowej serii.")

    results = [result] + ([terminal_result] if terminal_result is not None else [])
    successful = all(current.wasSuccessful() for current in results)
    coverage = [
        {"id": "EM-01", "level": "live-api"},
        {"id": "EM-02", "level": "live-api"},
        {"id": "EM-03", "level": "live-api"},
        {"id": "EM-04", "level": "published-state-only"},
        {"id": "EM-05", "level": "live-api"},
    ]
    if terminal_executed:
        coverage.append({"id": "EM-07", "level": "live-api-terminal"})
    report = {
        "schemaVersion": "rusd-issuer-p0-api-v1",
        "runId": prefix,
        "status": "PASS" if successful else "FAIL",
        "testsRun": sum(current.testsRun for current in results),
        "failures": sum(len(current.failures) for current in results),
        "errors": sum(len(current.errors) for current in results),
        "skipped": sum(len(current.skipped) for current in results),
        "durationSeconds": round(time.perf_counter() - started, 3),
        "terminalWindDownRequested": args.terminal_wind_down,
        "terminalWindDownExecuted": terminal_executed,
        "coverage": coverage,
    }
    output = Path(__file__).resolve().parents[1] / "test-results" / f"{prefix}.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"\nRaport: {output}")
    return 0 if successful else 1


if __name__ == "__main__":
    sys.exit(main())
