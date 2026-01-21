# qfc-consensus-spin

SPIN consensus.

SPIN works by having a list of authorities A who are expected to roughly
agree on the current time. Time is divided up into discrete slots of t
seconds each. Several slots make up a session, and every session's author is
selected by formula A[session_idx % |A|].

The author is allowed to issue N blocks but not more during that session,
and it will be built upon the longest valid chain that has been seen.

Blocks from future steps will be either deferred or rejected depending on how
far in the future they are.

NOTE: SPIN itself is designed to be generic over the crypto used.
