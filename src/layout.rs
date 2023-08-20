use keyberon::action::k;
use keyberon::action::Action::NoOp;
use keyberon::key_code::KeyCode::*;
use keyberon::layout::Layers;

#[rustfmt::skip]
pub static RIGHT_LAYER: Layers<9, 6, 1, ()> = [
    [
        [NoOp,    k(F7),   k(F8),    k(F9),   k(F10),    k(F11),      k(F12),   k(Delete), k(Insert)],
        [k(Kb7),    k(Kb8),   k(Kb9),    k(Kb0),   k(Minus),    k(Equal), NoOp, k(BSpace),   k(Home)],
        [k(Y),    k(U),   k(I),    k(O),   k(P),    k(LBracket),      k(RBracket),   k(Bslash), k(Y)],
        [k(H),    k(J),   k(K),    k(L),   k(SColon),    k(Quote),      NoOp, k(Enter),   k(Y)],
        [k(N),     k(M),    k(Comma), k(Dot), k(Slash), NoOp, k(RShift),       k(Up),  k(PgDown)],
        [NoOp, k(Space), k(LAlt),  k(RGui),  k(RGui), NoOp, k(Left),   k(Down),     k(Right)],
    ]
];
