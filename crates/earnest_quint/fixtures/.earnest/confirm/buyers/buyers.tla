---- MODULE buyers ----
EXTENDS Integers, Sequences, FiniteSets, TLC

VARIABLES lastBuyer, previousPeer, noise, log, controls

vars == <<lastBuyer, previousPeer, noise, log, controls>>

Fault(message) == Assert(FALSE, message)
EmptyMap == [key \in {} |-> key]
MapSet(map, key, value) == [other \in (DOMAIN map) \cup {key} |-> IF other = key THEN value ELSE map[other]]
MapMustGet(map, key) == IF key \in DOMAIN map THEN map[key] ELSE Fault("a key, a position or a case that is not there")
MapReplace(map, key, value) == IF key \in DOMAIN map THEN [map EXCEPT ![key] = value] ELSE Fault("a key, a position or a case that is not there")

InRange ==
    /\ TRUE
    /\ TRUE
    /\ (noise \in BOOLEAN)
    /\ TRUE
    /\ TRUE

Init ==
    /\ (((lastBuyer = "") /\ (previousPeer = "") /\ (noise = FALSE) /\ (log = 0) /\ (controls = MapSet(MapSet(EmptyMap, "ann", FALSE), "bob", FALSE))))

Action_reachDeadline ==
    \E buyer \in {"ann", "bob"} :
        IF InRange
        THEN /\ lastBuyer' = buyer
             /\ previousPeer' = lastBuyer
             /\ controls' = MapReplace(controls, buyer, TRUE)
             /\ UNCHANGED <<noise, log>>
        ELSE FALSE

Action_chatter ==
    IF InRange
    THEN /\ noise' = (~noise)
         /\ UNCHANGED <<lastBuyer, previousPeer, log, controls>>
    ELSE FALSE

End_stops_anywhere ==
    TRUE

Stop ==
    IF (~InRange) \/ End_stops_anywhere
    THEN UNCHANGED vars
    ELSE FALSE

Next ==
    \/ Action_reachDeadline
    \/ Action_chatter
    \/ Stop

Rule_parts ==
    InRange => (log = 0)

Rule_allForms ==
    InRange => (log = 0)

Fair_reachDeadline_1 ==
    \E buyer \in {"ann"} :
        IF InRange
        THEN /\ lastBuyer' = buyer
             /\ previousPeer' = lastBuyer
             /\ controls' = MapReplace(controls, buyer, TRUE)
             /\ UNCHANGED <<noise, log>>
        ELSE FALSE

Fair_reachDeadline_2 ==
    \E buyer \in {"bob"} :
        IF InRange
        THEN /\ lastBuyer' = buyer
             /\ previousPeer' = lastBuyer
             /\ controls' = MapReplace(controls, buyer, TRUE)
             /\ UNCHANGED <<noise, log>>
        ELSE FALSE

Fair_reachDeadline_3 ==
    \E buyer \in {"ann"} :
        IF InRange
        THEN /\ lastBuyer' = buyer
             /\ previousPeer' = lastBuyer
             /\ controls' = MapReplace(controls, buyer, TRUE)
             /\ UNCHANGED <<noise, log>>
        ELSE FALSE

Fair_reachDeadline_4 ==
    \E buyer \in {"ann", "bob"} :
        IF InRange
        THEN /\ lastBuyer' = buyer
             /\ previousPeer' = lastBuyer
             /\ controls' = MapReplace(controls, buyer, TRUE)
             /\ UNCHANGED <<noise, log>>
        ELSE FALSE

Fair_chatter_5 ==
    IF InRange
    THEN /\ noise' = (~noise)
         /\ UNCHANGED <<lastBuyer, previousPeer, log, controls>>
    ELSE FALSE

Spec ==
    Init /\ [][Next]_vars

Eventually_allReach ==
    (WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_1) /\ WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_2)) => (<>((\A item \in {"ann", "bob"} : MapMustGet(controls, item))))

Eventually_unfairAllReach ==
    <>((\A item \in {"ann", "bob"} : MapMustGet(controls, item)))

Eventually_noisy ==
    (WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_1) /\ WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_2)) => (<>(noise))

Eventually_annReaches ==
    (WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_1)) => (<>(MapMustGet(controls, "ann")))

Eventually_bobReaches ==
    (WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_1)) => (<>(MapMustGet(controls, "bob")))

Eventually_overPeer ==
    (WF_<<previousPeer>>(Fair_reachDeadline_3)) => (<>(MapMustGet(controls, "ann")))

Eventually_parts_2 ==
    (WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_1) /\ WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_2)) => ((MapMustGet(controls, "ann")) ~> ((\A item \in {"ann", "bob"} : MapMustGet(controls, item))))

Eventually_stepOverSome ==
    (WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_4 \/ Fair_chatter_5)) => (<>((\A item \in {"ann", "bob"} : MapMustGet(controls, item))))

Eventually_allForms_2 ==
    (WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_1) /\ WF_<<controls, lastBuyer, previousPeer>>(Fair_reachDeadline_2)) => (<>((\A item \in {"ann", "bob"} : MapMustGet(controls, item))))

====
