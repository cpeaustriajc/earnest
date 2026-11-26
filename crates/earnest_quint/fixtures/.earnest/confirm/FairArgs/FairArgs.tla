---- MODULE FairArgs ----
EXTENDS Integers, Sequences, FiniteSets, TLC

VARIABLES done

vars == <<done>>

Fault(message) == Assert(FALSE, message)
EmptyMap == [key \in {} |-> key]
MapSet(map, key, value) == [other \in (DOMAIN map) \cup {key} |-> IF other = key THEN value ELSE map[other]]
MapMustGet(map, key) == IF key \in DOMAIN map THEN map[key] ELSE Fault("a key, a position or a case that is not there")
MapReplace(map, key, value) == IF key \in DOMAIN map THEN [map EXCEPT ![key] = value] ELSE Fault("a key, a position or a case that is not there")

InRange ==
    /\ TRUE

Init ==
    /\ ((done = MapSet(MapSet(EmptyMap, 1, FALSE), 2, FALSE)))

Action_finish ==
    \E b \in {1, 2} :
        IF InRange /\ (~MapMustGet(done, b))
        THEN /\ done' = MapReplace(done, b, TRUE)
        ELSE FALSE

End_stops_anywhere ==
    TRUE

Stop ==
    IF (~InRange) \/ End_stops_anywhere
    THEN UNCHANGED vars
    ELSE FALSE

Next ==
    \/ Action_finish
    \/ Stop

Fair_finish_1 ==
    \E b \in {1} :
        IF InRange /\ (~MapMustGet(done, b))
        THEN /\ done' = MapReplace(done, b, TRUE)
        ELSE FALSE

Fair_finish_2 ==
    \E b \in {2} :
        IF InRange /\ (~MapMustGet(done, b))
        THEN /\ done' = MapReplace(done, b, TRUE)
        ELSE FALSE

Spec ==
    Init /\ [][Next]_vars

Eventually_EveryBuyerFinishes ==
    (WF_vars(Fair_finish_1) /\ WF_vars(Fair_finish_2)) => (<>(MapMustGet(done, 1)))

Eventually_EveryBuyerFinishes_2 ==
    (WF_vars(Fair_finish_1) /\ WF_vars(Fair_finish_2)) => (<>(MapMustGet(done, 2)))

Eventually_OnlyBuyerOne ==
    (WF_vars(Fair_finish_1)) => (<>(MapMustGet(done, 1)))

Eventually_OnlyBuyerOne_2 ==
    (WF_vars(Fair_finish_1)) => (<>(MapMustGet(done, 2)))

====
