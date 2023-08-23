use keyberon::action::k;
use keyberon::action::Action::NoOp;
use keyberon::key_code::KeyCode::*;
use keyberon::layout::Layers;

#[rustfmt::skip]
pub static LAYER: Layers<18, 6, 1, ()> = [
    [
        [NoOp,      NoOp,        k(Escape),   k(F1),   k(F2),   k(F3),          k(F4),  k(F5),    k(F6),  NoOp,       k(F7),    k(F8),    k(F9),   k(F10),    k(F11),      k(F12),      k(Delete), k(Insert)],
        [k(Kp1), k(Kp2), k(Grave),    k(Kb1),  k(Kb2),  k(Kb3),         k(Kb4), k(Kb5),   k(Kb6), k(Kb7), k(Kb8),   k(Kb9),   k(Kb0),  k(Minus),  k(Equal),    NoOp,            k(BSpace), k(Home)  ],
        [k(Kp3), k(Kp4), k(Tab),      k(Q),    k(W),    k(E),           k(R),   k(T),     NoOp,       k(Y),   k(U),     k(I),     k(O),    k(P),      k(LBracket), k(RBracket), k(Bslash), k(Y)     ],
        [k(Kp5), k(Kp6), k(CapsLock), k(A),    k(S),    k(D),           k(F),   k(G),     NoOp,       k(H),   k(J),     k(K),     k(L),    k(SColon), k(Quote),    NoOp,            k(Enter),  k(Y)     ],
        [k(Kp7), k(Kp8), k(LShift),   NoOp,        k(Z),    k(X),           k(C),   k(V),     k(B),   k(N),   k(M),     k(Comma), k(Dot),  k(Slash),  NoOp,            k(RShift),   k(Up),     k(PgDown)],
        [k(Kp9), k(Kp0), k(LCtrl),    k(LGui), k(LAlt), k(Application), NoOp,       k(Space), NoOp,       NoOp,       k(Space), k(LAlt),  k(RGui), k(RGui),   NoOp,            k(Left),     k(Down),   k(Right) ],
    ]
];

#[rustfmt::skip]
pub static LEFT_LAYER: Layers<9, 6, 1, ()> = [
    [
        [NoOp,      NoOp,       k(Escape),   k(F1),   k(F2),   k(F3),          k(F4),  k(F5),    k(F6) ],
        [k(F1), k(F2),  k(Grave),    k(Kb1),  k(Kb2),  k(Kb3),         k(Kb4), k(Kb5),   k(Kb6)],
        [k(F3), k(F4),  k(Tab),      k(Q),    k(W),    k(E),           k(R),   k(T),     NoOp      ],
        [k(F5), k(F6),  k(CapsLock), k(A),    k(S),    k(D),           k(F),   k(G),     NoOp      ],
        [k(F7), k(F8),  k(LShift),   NoOp,        k(Z),    k(X),           k(C),   k(V),     k(B)  ],
        [k(F9), k(F10), k(LCtrl),    k(LGui), k(LAlt), k(Application), NoOp,       k(Space), NoOp      ],
    ]
];

#[rustfmt::skip]
pub static RIGHT_LAYER: Layers<9, 6, 1, ()> = [
    [
        [NoOp,       k(F7),    k(F8),    k(F9),   k(F10),    k(F11),      k(F12),      k(Delete), k(Insert)],
        [k(Kb7), k(Kb8),   k(Kb9),   k(Kb0),  k(Minus),  k(Equal),    NoOp,            k(BSpace), k(Home)  ],
        [k(Y),   k(U),     k(I),     k(O),    k(P),      k(LBracket), k(RBracket), k(Bslash), k(Y)     ],
        [k(H),   k(J),     k(K),     k(L),    k(SColon), k(Quote),    NoOp,            k(Enter),  k(Y)     ],
        [k(N),   k(M),     k(Comma), k(Dot),  k(Slash),  NoOp,            k(RShift),   k(Up),     k(PgDown)],
        [NoOp,       k(Space), k(LAlt),  k(RGui), k(RGui),   NoOp,            k(Left),     k(Down),   k(Right) ],
    ]
];
