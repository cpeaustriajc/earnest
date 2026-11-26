---- MODULE fairshapes ----
EXTENDS Integers, Sequences, FiniteSets, TLC

VARIABLES x, y

vars == <<x, y>>

InRange ==
    /\ TRUE
    /\ TRUE

Init ==
    /\ (((x = 0) /\ (y = 0)))

Action_up ==
    IF InRange /\ (x < 2)
    THEN /\ x' = (x + 1)
         /\ UNCHANGED <<y>>
    ELSE FALSE

Action_flip ==
    IF InRange /\ (y < 1)
    THEN /\ y' = 1
         /\ UNCHANGED <<x>>
    ELSE FALSE

Action_rest ==
    IF InRange
    THEN /\ UNCHANGED <<x, y>>
    ELSE FALSE

End_stops_anywhere ==
    TRUE

Stop ==
    IF (~InRange) \/ End_stops_anywhere
    THEN UNCHANGED vars
    ELSE FALSE

Next ==
    \/ Action_up
    \/ Action_flip
    \/ Action_rest
    \/ Stop

Rule_parts ==
    InRange => (x <= 2)

Rule_infix ==
    InRange => (x <= 2)

Rule_nestedParts ==
    InRange => (y <= 1)

Spec ==
    Init /\ [][Next]_vars

Eventually_overPair ==
    (WF_vars(Action_up)) => (<>((x = 2)))

Eventually_overRecord ==
    (WF_vars(Action_up)) => (<>((x = 2)))

Eventually_overNested ==
    (WF_vars(Action_up)) => (<>((x = 2)))

Eventually_overList ==
    (WF_vars(Action_up)) => (<>((x = 2)))

Eventually_overSetOfOne ==
    (WF_vars(Action_up)) => (<>((x = 2)))

Eventually_parts_2 ==
    (WF_vars(Action_up) /\ WF_vars(Action_flip)) => (<>((x = 2)))

Eventually_parts_3 ==
    (WF_vars(Action_up) /\ WF_vars(Action_flip)) => (((x = 1)) ~> ((y = 1)))

Eventually_parts_4 ==
    <>((y = 1))

Eventually_infix_2 ==
    (WF_vars(Action_up) /\ WF_vars(Action_flip)) => (<>((y = 1)))

Eventually_infix_3 ==
    <>((x = 0))

Eventually_nestedParts_2 ==
    (WF_vars(Action_up)) => (<>((x = 2)))

Eventually_nestedParts_3 ==
    (WF_vars(Action_flip)) => (<>((x = 2)))

Eventually_startsAtOne ==
    <>((x = 1))

Eventually_eachTarget ==
    (WF_vars(Action_up) /\ WF_vars(Action_flip)) => (<>((x = 1)))

Eventually_eachTarget_2 ==
    (WF_vars(Action_up) /\ WF_vars(Action_flip)) => (<>((x = 2)))

Eventually_fairEach ==
    (WF_vars(Action_up)) => (<>((x = 2)))

Eventually_unfairEach ==
    <>((x = 1))

Eventually_unfairEach_2 ==
    <>((x = 2))

====
