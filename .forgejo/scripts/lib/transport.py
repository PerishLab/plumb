import http.client
import time
from urllib.parse import urlsplit


def remaining(deadline):
    value = deadline - time.monotonic()
    if value <= 0:
        raise TimeoutError("inventory transfer deadline exceeded")
    return min(20, value)


def exchange(address, method, headers, body):
    url = urlsplit(address)
    deadline = time.monotonic() + 180
    connection = http.client.HTTPSConnection(url.hostname, url.port, timeout=5)
    try:
        connection.connect()
        socket = connection.sock
        socket.settimeout(remaining(deadline))
        connection.putrequest(method, url.path, skip_host=True, skip_accept_encoding=True)
        for name, value in headers.items():
            connection.putheader(name, value)
        connection.endheaders()
        for start in range(0, len(body), 1024 * 1024):
            socket.settimeout(remaining(deadline))
            connection.send(memoryview(body)[start:start + 1024 * 1024])
        socket.settimeout(remaining(deadline))
        response = connection.getresponse()
        with response:
            parts = []
            size = 0
            while method != "HEAD":
                socket.settimeout(remaining(deadline))
                part = response.read1(1024 * 1024 if response.status == 200 else 65537)
                if not part:
                    break
                parts.append(part)
                size += len(part)
                if response.status != 200 and size > 65536:
                    raise http.client.HTTPException("inventory error response exceeds limit")
            if method != "HEAD" and response.length not in (None, 0):
                raise http.client.IncompleteRead(b"", response.length)
            remaining(deadline)
            return response.status, dict(response.getheaders()), b"".join(parts)
    finally:
        connection.close()
