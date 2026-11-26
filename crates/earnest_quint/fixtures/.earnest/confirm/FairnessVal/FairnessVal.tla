---- MODULE FairnessVal ----
EXTENDS Integers, Sequences, FiniteSets, TLC

VARIABLES st

vars == <<st>>

InRange ==
    /\ TRUE

Init ==
    /\ ((st = [n |-> 0]))

Action_tick ==
    IF InRange /\ (st.n < 2)
    THEN /\ st' = [n |-> (st.n + 1)]
    ELSE FALSE

Action_stay ==
    IF InRange
    THEN /\ UNCHANGED <<st>>
    ELSE FALSE

End_stops_anywhere ==
    TRUE

Stop ==
    IF (~InRange) \/ End_stops_anywhere
    THEN UNCHANGED vars
    ELSE FALSE

Next ==
    \/ Action_tick
    \/ Action_stay
    \/ Stop

Spec ==
    Init /\ [][Next]_vars

Eventually_ReachesTwo ==
    (WF_vars(Action_tick)) => (<>((st.n = 2)))

====
