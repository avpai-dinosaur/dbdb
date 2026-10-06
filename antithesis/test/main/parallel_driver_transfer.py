#!/usr/bin/env python3
"""Moves a single unit of value back and forth between two accounts."""

import sys

from antithesis.assertions import always, reachable

from helper_dbdb import Dbdb, DbdbError, DbdbGone


def main():
    with Dbdb() as db:
        a_value = db.get("accountA")
        always(a_value is not None, "accountA exists in db", {"accountA": a_value})

        b_value = db.get("accountB")
        always(b_value is not None, "accountB exists in db", {"accountB": b_value})

        if a_value is None or b_value is None:
            return 0

        count = (a_value == "full") + (b_value == "full")
        always(
            count == 1,
            "total value between accountA and accountB is 1",
            {"accountA": a_value, "accountB": b_value, "count": count},
        )

        if count != 1:
            return 0

        if a_value == "full":
            db.set("accountA", "empty")
            db.set("accountB", "full")
            reachable("transfer from a to b", {})
        else:
            db.set("accountA", "full")
            db.set("accountB", "empty")
            reachable("transfer from b to a", {})

        db.commit()
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (DbdbGone, DbdbError) as e:
        print("parallel_driver_transfer: transient: %s" % e, file=sys.stderr, flush=True)
        sys.exit(0)
