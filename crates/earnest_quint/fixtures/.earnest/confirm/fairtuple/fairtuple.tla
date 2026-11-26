---- MODULE fairtuple ----
EXTENDS Integers, Sequences, FiniteSets, TLC

VARIABLES x, done

vars == <<x, done>>

InRange ==
    /\ TRUE
    /\ (done \in BOOLEAN)

Init ==
    /\ (((x = 0) /\ (done = FALSE)))

Action_tick ==
    IF InRange /\ (x < 3)
    THEN /\ x' = (x + 1)
         /\ done' = ((x + 1) = 3)
    ELSE FALSE

Action_rest ==
    IF InRange
    THEN /\ UNCHANGED <<x, done>>
    ELSE FALSE

End_stops_anywhere ==
    TRUE

Stop ==
    IF (~InRange) \/ End_stops_anywhere
    THEN UNCHANGED vars
    ELSE FALSE

Next ==
    \/ Action_tick
    \/ Action_rest
    \/ Stop

Rule_neverFour ==
    InRange => (~(x = 4))

Rule_neverTwo ==
    InRange => (~(x = 2))

Spec ==
    Init /\ [][Next]_vars

Eventually_finishes ==
    (WF_vars(Action_tick)) => (<>(done))

Eventually_finishesThroughVals ==
    (WF_vars(Action_tick)) => (<>(done))

Eventually_finishesBackwards ==
    (WF_vars(Action_tick)) => (<>((x = 3)))

Eventually_unfairFinishes ==
    <>(done)

Eventually_leavesZero ==
    (WF_vars(Action_tick)) => (<>((~(x = 0))))

Eventually_mayStayAtZero ==
    <>((~(x = 0)))

====
