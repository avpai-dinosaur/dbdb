"""Client for the dbdb CLI's line protocol."""

import os
import subprocess

DEFAULT_PATH = os.environ.get("DBDB_PATH", "/data/dbdb.db")
DEFAULT_CLI = os.environ.get("DBDB_CLI", "dbdb")


class DbdbError(RuntimeError):
    """The database answered ERR, or said something we don't understand."""


class DbdbGone(RuntimeError):
    """The CLI process went away mid-conversation.

    Expected under fault injection -- Antithesis can kill it at any point -- so
    callers should treat this as a transient condition and not as a bug.
    """


class Dbdb:
    def __init__(self, path=DEFAULT_PATH, cli=DEFAULT_CLI):
        self.proc = subprocess.Popen(
            [cli, path, "repl"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            text=True,
            bufsize=1,          # line buffered: we need each command to go out on its own
        )

    def _command(self, line):
        """Sends one command and returns its single response line."""
        if self.proc.poll() is not None:
            raise DbdbGone("dbdb exited with %s before the command was sent" % self.proc.returncode)
        try:
            self.proc.stdin.write(line + "\n")
            self.proc.stdin.flush()
        except (BrokenPipeError, ValueError) as e:
            raise DbdbGone("dbdb's stdin closed: %s" % e) from e

        response = self.proc.stdout.readline()
        if response == "":
            raise DbdbGone("dbdb closed stdout (exit %s)" % self.proc.poll())
        response = response.rstrip("\n")
        if response.startswith("ERR "):
            raise DbdbError(response[4:])
        return response

    def get(self, key):
        """Returns the value as a str, or None if the key is absent."""
        response = self._command("get " + key)
        if response == "NOTFOUND":
            return None
        if response.startswith("VALUE "):
            return response[len("VALUE "):]
        raise DbdbError("unexpected response to get: %r" % response)

    def set(self, key, value):
        self._expect_ok("set %s %s" % (key, value))

    def delete(self, key):
        self._expect_ok("delete " + key)

    def commit(self):
        self._expect_ok("commit")

    def _expect_ok(self, line):
        response = self._command(line)
        if response != "OK":
            raise DbdbError("unexpected response to %r: %r" % (line, response))

    def close(self):
        """Ends the session. Uncommitted writes are discarded and the lock released."""
        if self.proc.poll() is None:
            try:
                self.proc.stdin.write("exit\n")
                self.proc.stdin.flush()
                self.proc.stdin.close()
            except (BrokenPipeError, ValueError, OSError):
                pass            # already gone; the wait below reaps it
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()
        return False
