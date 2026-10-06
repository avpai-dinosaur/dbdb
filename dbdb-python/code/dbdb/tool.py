from __future__ import print_function
import sys

import dbdb


OK = 0
BAD_ARGS = 1
BAD_VERB = 2
BAD_KEY = 3


def usage():
    print("Usage:", file=sys.stderr)
    print("\tpython -m dbdb.tool DBNAME get KEY", file=sys.stderr)
    print("\tpython -m dbdb.tool DBNAME set KEY VALUE", file=sys.stderr)
    print("\tpython -m dbdb.tool DBNAME delete KEY", file=sys.stderr)
    print("\tpython -m dbdb.tool DBNAME [repl]", file=sys.stderr)


def _next_token(line, idx):
    """Splits off the next space-delimited token, returning it and the new idx.

    Returns '' once the line is exhausted.
    """
    while idx < len(line) and line[idx] == ' ':
        idx += 1
    start = idx
    while idx < len(line) and line[idx] != ' ':
        idx += 1
    return line[start:idx], idx


def _rest_of_line(line, idx):
    """The remainder of the line, minus the spaces separating it from the
    previous token. Keeps interior spaces, so a value may contain them."""
    while idx < len(line) and line[idx] == ' ':
        idx += 1
    return line[idx:]


def _one_line(text):
    """A response is exactly one line, so an error message carrying a newline
    would desynchronize the caller's parser."""
    return text.replace('\n', ' ').replace('\r', ' ')


def repl(db, stdin=None, stdout=None):
    """Reads one command per line and writes exactly one response line each.

    This is the surface the Antithesis harness drives; see
    ../../antithesis/CONTRACT.md for the normative description. Unlike the
    one-shot verbs above, set and delete here do not commit -- the first one
    takes the file lock and holds it until an explicit commit, so several writes
    can be made atomic. Reaching exit or EOF discards uncommitted writes.
    """
    stdin = stdin if stdin is not None else sys.stdin
    stdout = stdout if stdout is not None else sys.stdout

    def respond(text):
        # The caller writes a command and blocks on the reply, so an unflushed
        # response is a deadlock.
        stdout.write(text + '\n')
        stdout.flush()

    while True:
        raw = stdin.readline()
        if not raw:
            return OK                       # EOF behaves like exit
        line = raw.rstrip('\n')
        if line.endswith('\r'):             # tolerate CRLF
            line = line[:-1]

        verb, idx = _next_token(line, 0)
        if not verb:
            continue                        # blank line

        try:
            if verb in ('exit', 'quit'):
                return OK
            elif verb == 'get':
                key, idx = _next_token(line, idx)
                if not key:
                    respond('ERR get requires a key')
                    continue
                try:
                    respond('VALUE ' + db[key])
                except KeyError:
                    respond('NOTFOUND')
            elif verb == 'set':
                key, idx = _next_token(line, idx)
                if not key or idx >= len(line):
                    respond('ERR set requires a key and a value')
                    continue
                db[key] = _rest_of_line(line, idx)
                respond('OK')
            elif verb == 'delete':
                key, idx = _next_token(line, idx)
                if not key:
                    respond('ERR delete requires a key')
                    continue
                try:
                    del db[key]
                except KeyError:
                    pass                    # deleting an absent key is a no-op
                respond('OK')
            elif verb == 'commit':
                db.commit()
                respond('OK')
            else:
                respond('ERR unknown command: ' + _one_line(verb))
        except Exception as e:              # noqa: BLE001 -- see below
            # Report and keep going rather than dying: an abrupt exit would read
            # as a crash to whatever is supervising this process. A failure
            # part-way through a transaction leaves the lock held, so the caller
            # can still retry, commit, or exit.
            respond('ERR ' + _one_line(str(e) or e.__class__.__name__))


def main(argv):
    if len(argv) == 2 or (len(argv) == 3 and argv[2] == 'repl'):
        db = dbdb.connect(argv[1])
        try:
            return repl(db)
        finally:
            db.close()   # drops the lock; uncommitted writes are discarded
    if not (4 <= len(argv) <= 5):
        usage()
        return BAD_ARGS
    dbname, verb, key, value = (argv[1:] + [None])[:4]
    if verb not in {'get', 'set', 'delete'}:
        usage()
        return BAD_VERB
    db = dbdb.connect(dbname)
    try:
        if verb == 'get':
            sys.stdout.write(db[key])
        elif verb == 'set':
            db[key] = value
            db.commit()
        else:
            del db[key]
            db.commit()
    except KeyError:
        print("Key not found", file=sys.stderr)
        return BAD_KEY
    return OK


if __name__ == '__main__':
    sys.exit(main(sys.argv))
