import http.client
import unittest
from unittest.mock import MagicMock, patch

from lib.transport import exchange, remaining


class Transport(unittest.TestCase):
    def connection(self, status=200, parts=()):
        connection = MagicMock()
        response = connection.getresponse.return_value
        response.status = status
        response.length = 0
        response.read1.side_effect = [*parts, b""]
        response.getheaders.return_value = [("X-Amz-Checksum-Sha256", "fixture")]
        return connection

    def test_https_upload_is_chunked_without_redirects_or_packages(self):
        connection = self.connection()
        body = b"x" * (1024 * 1024 + 3)
        with patch("lib.transport.http.client.HTTPSConnection", return_value=connection) as factory:
            result = exchange("https://inventory.test/bucket/object", "PUT",
                              {"host": "inventory.test", "content-length": str(len(body))}, body)
        factory.assert_called_once_with("inventory.test", None, timeout=5)
        connection.putrequest.assert_called_once_with("PUT", "/bucket/object",
                                                       skip_host=True, skip_accept_encoding=True)
        self.assertEqual([len(call.args[0]) for call in connection.send.call_args_list], [1024 * 1024, 3])
        self.assertEqual(result, (200, {"X-Amz-Checksum-Sha256": "fixture"}, b""))
        connection.close.assert_called_once()

    def test_head_does_not_download_payload(self):
        connection = self.connection()
        with patch("lib.transport.http.client.HTTPSConnection", return_value=connection):
            exchange("https://inventory.test/object", "HEAD", {}, b"")
        connection.getresponse.return_value.read1.assert_not_called()

    def test_get_collects_actual_response_bytes(self):
        connection = self.connection(parts=(b"one", b"two"))
        with patch("lib.transport.http.client.HTTPSConnection", return_value=connection):
            self.assertEqual(exchange("https://inventory.test/object", "GET", {}, b"")[2], b"onetwo")

    def test_errors_have_a_bounded_response(self):
        connection = self.connection(status=500, parts=(b"x" * 65537,))
        with patch("lib.transport.http.client.HTTPSConnection", return_value=connection):
            with self.assertRaises(http.client.HTTPException):
                exchange("https://inventory.test/object", "GET", {}, b"")
        connection.close.assert_called_once()

    def test_partial_response_is_not_success(self):
        connection = self.connection(parts=(b"partial",))
        connection.getresponse.return_value.length = 20
        with patch("lib.transport.http.client.HTTPSConnection", return_value=connection):
            with self.assertRaises(http.client.IncompleteRead):
                exchange("https://inventory.test/object", "GET", {}, b"")

    def test_progressing_transfer_may_exceed_the_retired_sixty_second_cap(self):
        connection = self.connection(parts=(b"body",))
        with patch("lib.transport.http.client.HTTPSConnection", return_value=connection):
            with patch("lib.transport.time.monotonic", side_effect=[0, 1, 2, 60, 69, 70]):
                self.assertEqual(exchange("https://inventory.test/object", "GET", {}, b"")[2], b"body")

    def test_deadline_bounds_all_chunks_not_only_socket_idle(self):
        connection = self.connection(parts=(b"one", b"two"))
        with patch("lib.transport.http.client.HTTPSConnection", return_value=connection):
            with patch("lib.transport.time.monotonic", side_effect=[0, 1, 2, 3, 181]):
                with self.assertRaises(TimeoutError):
                    exchange("https://inventory.test/object", "GET", {}, b"")
        connection.close.assert_called_once()
        with patch("lib.transport.time.monotonic", return_value=170):
            self.assertEqual(remaining(180), 10)

    def test_tls_or_connection_failure_closes_without_retry(self):
        connection = self.connection()
        connection.connect.side_effect = OSError("private details")
        with patch("lib.transport.http.client.HTTPSConnection", return_value=connection):
            with self.assertRaises(OSError):
                exchange("https://inventory.test/object", "GET", {}, b"")
        connection.connect.assert_called_once()
        connection.close.assert_called_once()


if __name__ == "__main__":
    unittest.main()
