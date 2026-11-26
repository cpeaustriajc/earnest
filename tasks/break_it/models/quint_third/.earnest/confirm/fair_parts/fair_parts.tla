---- MODULE fair_parts ----
EXTENDS Integers, Sequences, FiniteSets, TLC

VARIABLES paid, last

vars == <<paid, last>>

Fault(message) == Assert(FALSE, message)
EmptyMap == [key \in {} |-> key]
MapSet(map, key, value) == [other \in (DOMAIN map) \cup {key} |-> IF other = key THEN value ELSE map[other]]
MapMustGet(map, key) == IF key \in DOMAIN map THEN map[key] ELSE Fault("a key, a position or a case that is not there")
MapReplace(map, key, value) == IF key \in DOMAIN map THEN [map EXCEPT ![key] = value] ELSE Fault("a key, a position or a case that is not there")

InRange ==
    /\ TRUE
    /\ TRUE

Init ==
    /\ (((paid = MapSet(MapSet(EmptyMap, 1, FALSE), 2, FALSE)) /\ (last = "")))

Action_pay ==
    \E r \in {[what |-> "a", who |-> 1], [what |-> "b", who |-> 2]} :
        IF InRange /\ (~MapMustGet(paid, r.who))
        THEN /\ paid' = MapReplace(paid, r.who, TRUE)
             /\ UNCHANGED <<last>>
        ELSE FALSE

Action_idle ==
    IF InRange
    THEN /\ UNCHANGED <<paid, last>>
    ELSE FALSE

End_stops_anywhere ==
    TRUE

Stop ==
    IF (~InRange) \/ End_stops_anywhere
    THEN UNCHANGED vars
    ELSE FALSE

Next ==
    \/ Action_pay
    \/ Action_idle
    \/ Stop

Fair_pay_1 ==
    \E r \in {[what |-> "a", who |-> 1]} :
        IF InRange /\ (~MapMustGet(paid, r.who))
        THEN /\ paid' = MapReplace(paid, r.who, TRUE)
             /\ UNCHANGED <<last>>
        ELSE FALSE

Fair_pay_2 ==
    \E r \in {[what |-> "b", who |-> 2]} :
        IF InRange /\ (~MapMustGet(paid, r.who))
        THEN /\ paid' = MapReplace(paid, r.who, TRUE)
             /\ UNCHANGED <<last>>
        ELSE FALSE

Spec ==
    Init /\ [][Next]_vars

Eventually_payEach ==
    (WF_<<paid>>(Fair_pay_1) /\ WF_<<paid>>(Fair_pay_2)) => (<>(MapMustGet(paid, 1)))

Eventually_payEach_2 ==
    (WF_<<paid>>(Fair_pay_1) /\ WF_<<paid>>(Fair_pay_2)) => (<>(MapMustGet(paid, 2)))

Eventually_payOne ==
    (WF_<<paid>>(Fair_pay_1)) => (<>(MapMustGet(paid, 1)))

Eventually_payOne_2 ==
    (WF_<<paid>>(Fair_pay_1)) => (<>(MapMustGet(paid, 2)))

Eventually_payOneOnly ==
    (WF_<<paid>>(Fair_pay_1)) => (<>(MapMustGet(paid, 1)))

====
