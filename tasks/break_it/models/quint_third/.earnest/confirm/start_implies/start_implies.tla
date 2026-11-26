---- MODULE start_implies ----
EXTENDS Integers, Sequences, FiniteSets, TLC

VARIABLES x

vars == <<x>>

InRange ==
    /\ TRUE

Init ==
    /\ ((x = 0))

Action_step ==
    IF InRange /\ (x < 3)
    THEN /\ x' = (x + 1)
    ELSE FALSE

Action_step_2 ==
    IF InRange
    THEN /\ UNCHANGED <<x>>
    ELSE FALSE

End_stops_anywhere ==
    TRUE

Stop ==
    IF (~InRange) \/ End_stops_anywhere
    THEN UNCHANGED vars
    ELSE FALSE

Next ==
    \/ Action_step
    \/ Action_step_2
    \/ Stop

Rule_fromOne ==
    InRange => TRUE

Spec ==
    Init /\ [][Next]_vars

Eventually_fromZero ==
    <>((x = 3))

Eventually_fairFromZero ==
    (WF_vars(Next)) => (<>((x = 3)))

====
