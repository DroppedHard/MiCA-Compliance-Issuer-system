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


def main() -> int:
    parser = argparse.ArgumentParser(description="Mutujace scenariusze API P0 emitenta (EM-01--EM-05).")
    parser.add_argument("--issuer-url", default="http://127.0.0.1:3000")
    parser.add_argument("--bank-url", default="http://127.0.0.1:3100")
    parser.add_argument("--timeout", type=float, default=10.0)
    args = parser.parse_args()
    prefix = "api-p0-issuer-" + uuid.uuid4().hex[:12]
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(
        build_tests(Client(args.issuer_url, args.timeout), Client(args.bank_url, args.timeout), prefix)
    )
    started = time.perf_counter()
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    report = {
        "schemaVersion": "rusd-issuer-p0-api-v1",
        "runId": prefix,
        "status": "PASS" if result.wasSuccessful() else "FAIL",
        "testsRun": result.testsRun,
        "failures": len(result.failures),
        "errors": len(result.errors),
        "skipped": len(result.skipped),
        "durationSeconds": round(time.perf_counter() - started, 3),
        "coverage": [
            {"id": "EM-01", "level": "live-api"},
            {"id": "EM-02", "level": "live-api"},
            {"id": "EM-03", "level": "live-api"},
            {"id": "EM-04", "level": "published-state-only"},
            {"id": "EM-05", "level": "live-api"},
        ],
    }
    output = Path(__file__).resolve().parents[1] / "test-results" / f"{prefix}.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"\nRaport: {output}")
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
