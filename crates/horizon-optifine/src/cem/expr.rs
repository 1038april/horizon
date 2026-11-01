//! CEM animation expressions (`"head.rx": "sin(limb_swing * 0.6662) * limb_speed"`), compiled
//! once when the packs load into a flat stack program, then evaluated per entity per frame
//! without allocating or touching strings.
//!
//! Variables are slots: the caller resolves each name (`limb_swing`, `head.rx`, `var.x`...) to
//! an index into its own `f32` array when compiling. Booleans are 1 and 0, and any value but 0
//! is true. `if`, `&&` and `||` evaluate only the branch they take.

use std::fmt;

/// Deepest value stack a program may need.
const STACK: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub message: String,
    /// Byte offset in the expression.
    pub offset: usize,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} at {}", self.message, self.offset)
    }
}
impl std::error::Error for Error {}

fn err<T>(message: impl Into<String>, offset: usize) -> Result<T, Error> {
    Err(Error {
        message: message.into(),
        offset,
    })
}

/// A compiled expression.
#[derive(Clone, Debug)]
pub struct Expr {
    code: Vec<Op>,
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Const(f32),
    Slot(u32),
    Neg,
    Not,
    Bin(Bin),
    Call(Func, u8),
    /// Pops; jumps when the value is 0.
    JumpIfZero(u32),
    Jump(u32),
    /// The top as a boolean, 1 or 0.
    Bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Bin {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}
impl Bin {
    fn apply(self, a: f32, b: f32) -> f32 {
        let t = |c: bool| if c { 1.0 } else { 0.0 };
        match self {
            Bin::Add => a + b,
            Bin::Sub => a - b,
            Bin::Mul => a * b,
            Bin::Div => a / b,
            Bin::Rem => a % b,
            Bin::Lt => t(a < b),
            Bin::Le => t(a <= b),
            Bin::Gt => t(a > b),
            Bin::Ge => t(a >= b),
            Bin::Eq => t(a == b),
            Bin::Ne => t(a != b),
        }
    }
}

macro_rules! funcs {
    ($($name:literal => $f:ident ($min:literal, $max:expr)),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        enum Func { $($f),* }
        impl Func {
            fn named(name: &str) -> Option<(Func, usize, usize)> {
                Some(match name { $($name => (Func::$f, $min, $max),)* _ => return None })
            }
        }
    };
}
const MANY: usize = usize::MAX;
funcs! {
    "sin" => Sin(1, 1), "cos" => Cos(1, 1), "asin" => Asin(1, 1), "acos" => Acos(1, 1),
    "tan" => Tan(1, 1), "atan" => Atan(1, 1), "atan2" => Atan2(2, 2), "torad" => Torad(1, 1),
    "todeg" => Todeg(1, 1), "min" => Min(1, MANY), "max" => Max(1, MANY),
    "clamp" => Clamp(3, 3), "abs" => Abs(1, 1), "floor" => Floor(1, 1), "ceil" => Ceil(1, 1),
    "exp" => Exp(1, 1), "frac" => Frac(1, 1), "log" => Log(1, 1), "pow" => Pow(2, 2),
    "random" => Random(0, 1), "randomb" => Randomb(0, 1), "round" => Round(1, 1),
    "signum" => Signum(1, 1), "sqrt" => Sqrt(1, 1), "fmod" => Fmod(2, 2), "lerp" => Lerp(3, 3),
    "print" => Print(3, 3), "printb" => Printb(3, 3), "between" => Between(3, 3),
    "equals" => Equals(3, 3), "in" => In(2, MANY), "wrapdeg" => Wrapdeg(1, 1),
    "wraprad" => Wraprad(1, 1), "degdiff" => Degdiff(2, 2), "raddiff" => Raddiff(2, 2),
    "catch" => Catch(2, 3), "keyframe" => Keyframe(2, MANY), "keyframeloop" => Keyframeloop(2, MANY),
    "catmullrom" => Catmullrom(5, 5), "hermite" => Hermite(5, 5), "cubicbezier" => Cubicbezier(5, 5),
    "quadbezier" => Quadbezier(4, 4),
    // EMF's `nbt(key, test)` needs the entity's NBT, which the client model doesn't have.
    "nbt" => Nbt(2, 2),
    "easeinquad" => InQuad(3, 3), "easeoutquad" => OutQuad(3, 3), "easeinoutquad" => InOutQuad(3, 3),
    "easeincubic" => InCubic(3, 3), "easeoutcubic" => OutCubic(3, 3), "easeinoutcubic" => InOutCubic(3, 3),
    "easeinquart" => InQuart(3, 3), "easeoutquart" => OutQuart(3, 3), "easeinoutquart" => InOutQuart(3, 3),
    "easeinquint" => InQuint(3, 3), "easeoutquint" => OutQuint(3, 3), "easeinoutquint" => InOutQuint(3, 3),
    "easeinsine" => InSine(3, 3), "easeoutsine" => OutSine(3, 3), "easeinoutsine" => InOutSine(3, 3),
    "easeinexpo" => InExpo(3, 3), "easeoutexpo" => OutExpo(3, 3), "easeinoutexpo" => InOutExpo(3, 3),
    "easeincirc" => InCirc(3, 3), "easeoutcirc" => OutCirc(3, 3), "easeinoutcirc" => InOutCirc(3, 3),
    "easeinback" => InBack(3, 3), "easeoutback" => OutBack(3, 3), "easeinoutback" => InOutBack(3, 3),
    "easeinelastic" => InElastic(3, 3), "easeoutelastic" => OutElastic(3, 3),
    "easeinoutelastic" => InOutElastic(3, 3),
    "easeinbounce" => InBounce(3, 3), "easeoutbounce" => OutBounce(3, 3),
    "easeinoutbounce" => InOutBounce(3, 3),
}

const PI: f32 = std::f32::consts::PI;

fn wrap(x: f32, turn: f32) -> f32 {
    let r = x % turn;
    if r >= turn / 2.0 {
        r - turn
    } else if r < -turn / 2.0 {
        r + turn
    } else {
        r
    }
}

fn random(seed: Option<f32>) -> f32 {
    use std::cell::Cell;
    thread_local!(static STATE: Cell<u64> = const { Cell::new(0x9E37_79B9_7F4A_7C15) });
    let mut x = match seed {
        Some(s) => {
            u64::from(s.to_bits()).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03
        }
        None => STATE.with(|s| {
            let v = s.get().wrapping_add(0x9E37_79B9_7F4A_7C15);
            s.set(v);
            v
        }),
    };
    // splitmix64's finaliser.
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

fn bounce_out(t: f32) -> f32 {
    let (n, d) = (7.5625, 2.75);
    if t < 1.0 / d {
        n * t * t
    } else if t < 2.0 / d {
        let t = t - 1.5 / d;
        n * t * t + 0.75
    } else if t < 2.5 / d {
        let t = t - 2.25 / d;
        n * t * t + 0.9375
    } else {
        let t = t - 2.625 / d;
        n * t * t + 0.984375
    }
}

/// The easings.net curve of an `ease*` function at `t`; None for other functions.
fn ease(f: Func, t: f32) -> Option<f32> {
    use Func::*;
    let inout = |t: f32, p: i32| {
        if t < 0.5 {
            2f32.powi(p - 1) * t.powi(p)
        } else {
            1.0 - (-2.0 * t + 2.0).powi(p) / 2.0
        }
    };
    let (c1, c4, c5) = (1.70158, 2.0 * PI / 3.0, 2.0 * PI / 4.5);
    let (c2, c3) = (c1 * 1.525, c1 + 1.0);
    Some(match f {
        InQuad => t * t,
        OutQuad => 1.0 - (1.0 - t).powi(2),
        InOutQuad => inout(t, 2),
        InCubic => t.powi(3),
        OutCubic => 1.0 - (1.0 - t).powi(3),
        InOutCubic => inout(t, 3),
        InQuart => t.powi(4),
        OutQuart => 1.0 - (1.0 - t).powi(4),
        InOutQuart => inout(t, 4),
        InQuint => t.powi(5),
        OutQuint => 1.0 - (1.0 - t).powi(5),
        InOutQuint => inout(t, 5),
        InSine => 1.0 - (t * PI / 2.0).cos(),
        OutSine => (t * PI / 2.0).sin(),
        InOutSine => -((PI * t).cos() - 1.0) / 2.0,
        InExpo if t == 0.0 => 0.0,
        InExpo => 2f32.powf(10.0 * t - 10.0),
        OutExpo if t == 1.0 => 1.0,
        OutExpo => 1.0 - 2f32.powf(-10.0 * t),
        InOutExpo if t == 0.0 || t == 1.0 => t,
        InOutExpo if t < 0.5 => 2f32.powf(20.0 * t - 10.0) / 2.0,
        InOutExpo => (2.0 - 2f32.powf(-20.0 * t + 10.0)) / 2.0,
        InCirc => 1.0 - (1.0 - t * t).sqrt(),
        OutCirc => (1.0 - (t - 1.0).powi(2)).sqrt(),
        InOutCirc if t < 0.5 => (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0,
        InOutCirc => ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0,
        InBack => c3 * t.powi(3) - c1 * t * t,
        OutBack => 1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2),
        InOutBack if t < 0.5 => (2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2) / 2.0,
        InOutBack => ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0,
        InElastic | OutElastic | InOutElastic if t == 0.0 || t == 1.0 => t,
        InElastic => -(2f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin(),
        OutElastic => 2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0,
        InOutElastic if t < 0.5 => {
            -(2f32.powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()) / 2.0
        }
        InOutElastic => 2f32.powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin() / 2.0 + 1.0,
        InBounce => 1.0 - bounce_out(1.0 - t),
        OutBounce => bounce_out(t),
        InOutBounce if t < 0.5 => (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0,
        InOutBounce => (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0,
        _ => return None,
    })
}

impl Func {
    /// Whether two calls with the same arguments may give different results.
    fn varies(self, argc: usize) -> bool {
        matches!(self, Func::Random | Func::Randomb) && argc == 0
    }

    fn apply(self, a: &[f32]) -> f32 {
        use Func::*;
        let t = |c: bool| if c { 1.0 } else { 0.0 };
        let lerp = |k: f32, x: f32, y: f32| x + k * (y - x);
        match self {
            Sin => a[0].sin(),
            Cos => a[0].cos(),
            Asin => a[0].asin(),
            Acos => a[0].acos(),
            Tan => a[0].tan(),
            Atan => a[0].atan(),
            Atan2 => a[0].atan2(a[1]),
            Torad => a[0].to_radians(),
            Todeg => a[0].to_degrees(),
            Min => a.iter().copied().fold(f32::INFINITY, f32::min),
            Max => a.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            Clamp => a[0].max(a[1]).min(a[2]),
            Abs => a[0].abs(),
            Floor => a[0].floor(),
            Ceil => a[0].ceil(),
            Exp => a[0].exp(),
            Frac => a[0] - a[0].floor(),
            Log => a[0].ln(),
            Pow => a[0].powf(a[1]),
            Random => random(a.first().copied()),
            Randomb => t(random(a.first().copied()) >= 0.5),
            // Java's `Math.round`: halves go up.
            Round => (a[0] + 0.5).floor(),
            Signum if a[0] == 0.0 || a[0].is_nan() => a[0],
            Signum => a[0].signum(),
            Sqrt => a[0].sqrt(),
            Fmod => {
                let r = a[0] % a[1];
                if r != 0.0 && (r < 0.0) != (a[1] < 0.0) {
                    r + a[1]
                } else {
                    r
                }
            }
            Lerp => lerp(a[0], a[1], a[2]),
            // Printing to the log is a debugging aid: only the value matters here.
            Print | Printb => a[2],
            Between => t(a[1] <= a[0] && a[0] <= a[2]),
            Equals => t((a[0] - a[1]).abs() <= a[2]),
            In => t(a[1..].contains(&a[0])),
            Wrapdeg => wrap(a[0], 360.0),
            Wraprad => wrap(a[0], 2.0 * PI),
            Degdiff => wrap(a[1] - a[0], 360.0),
            Raddiff => wrap(a[1] - a[0], 2.0 * PI),
            Catch if a[0].is_finite() => a[0],
            Catch => a[1],
            Keyframe | Keyframeloop => {
                let frames = &a[1..];
                let n = frames.len();
                let k = a[0].abs();
                let k = if self == Keyframeloop {
                    k % n as f32
                } else {
                    k
                };
                let i = k.floor() as usize;
                if i + 1 >= n && self == Keyframe {
                    return frames[n - 1];
                }
                let i = i.min(n - 1);
                lerp(k - i as f32, frames[i], frames[(i + 1) % n])
            }
            // `Mth.catmullrom(k, p0, p1, p2, p3)`: from p1 to p2.
            Catmullrom => {
                let (k, p0, p1, p2, p3) = (a[0], a[1], a[2], a[3], a[4]);
                0.5 * (2.0 * p1
                    + (p2 - p0) * k
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * k * k
                    + (3.0 * p1 - p0 - 3.0 * p2 + p3) * k * k * k)
            }
            // From x to y with tangents z and w.
            Hermite => {
                let (k, x, y, m0, m1) = (a[0], a[1], a[2], a[3], a[4]);
                let (k2, k3) = (k * k, k * k * k);
                (2.0 * k3 - 3.0 * k2 + 1.0) * x
                    + (k3 - 2.0 * k2 + k) * m0
                    + (-2.0 * k3 + 3.0 * k2) * y
                    + (k3 - k2) * m1
            }
            // From x to y through control points z (and w).
            Cubicbezier => {
                let (k, x, y, z, w) = (a[0], a[1], a[2], a[3], a[4]);
                let j = 1.0 - k;
                j * j * j * x + 3.0 * j * j * k * z + 3.0 * j * k * k * w + k * k * k * y
            }
            Quadbezier => {
                let (k, x, y, z) = (a[0], a[1], a[2], a[3]);
                let j = 1.0 - k;
                j * j * x + 2.0 * j * k * z + k * k * y
            }
            Nbt => 0.0,
            // The `ease*` functions: `lerp` along their curve.
            _ => lerp(ease(self, a[0]).unwrap_or(a[0]), a[1], a[2]),
        }
    }
}

/// The parsed form, folded before it becomes code.
enum Node {
    Num(f32),
    Slot(u32),
    Neg(Box<Node>),
    Not(Box<Node>),
    Bin(Bin, Box<Node>, Box<Node>),
    And(Box<Node>, Box<Node>),
    Or(Box<Node>, Box<Node>),
    /// cond, value, [cond, value...], else
    If(Vec<Node>),
    Call(Func, Vec<Node>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tok<'a> {
    Num(f32),
    Name(&'a str),
    Sym(&'a str),
    End,
}

struct Parser<'a, 'r> {
    src: &'a str,
    pos: usize,
    tok: Tok<'a>,
    at: usize,
    resolve: &'r mut dyn FnMut(&str) -> Option<u32>,
}

impl<'a> Parser<'a, '_> {
    fn next(&mut self) -> Result<(), Error> {
        let b = self.src.as_bytes();
        while self.pos < b.len() && b[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        self.at = self.pos;
        let Some(&c) = b.get(self.pos) else {
            self.tok = Tok::End;
            return Ok(());
        };
        let start = self.pos;
        let word = |c: u8| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b':');
        if c.is_ascii_digit() || c == b'.' && b.get(start + 1).is_some_and(u8::is_ascii_digit) {
            while self.pos < b.len() && (b[self.pos].is_ascii_digit() || b[self.pos] == b'.') {
                self.pos += 1;
            }
            let text = &self.src[start..self.pos];
            self.tok = Tok::Num(match text.parse() {
                Ok(v) => v,
                Err(_) => return err(format!("bad number {text}"), start),
            });
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while self.pos < b.len() && word(b[self.pos]) {
                self.pos += 1;
            }
            self.tok = Tok::Name(&self.src[start..self.pos]);
        } else {
            let two = self.src.get(start..start + 2);
            let len = match two {
                Some("&&" | "||" | "<=" | ">=" | "==" | "!=") => 2,
                _ if b"+-*/%!<>(),".contains(&c) => 1,
                _ => return err(format!("unexpected {:?}", c as char), start),
            };
            self.pos += len;
            self.tok = Tok::Sym(&self.src[start..self.pos]);
        }
        Ok(())
    }

    fn eat(&mut self, sym: &str) -> Result<bool, Error> {
        if self.tok == Tok::Sym(sym) {
            self.next()?;
            return Ok(true);
        }
        Ok(false)
    }

    fn expect(&mut self, sym: &str) -> Result<(), Error> {
        if !self.eat(sym)? {
            return err(format!("expected {sym}"), self.at);
        }
        Ok(())
    }

    fn or(&mut self) -> Result<Node, Error> {
        let mut a = self.and()?;
        while self.eat("||")? {
            a = Node::Or(Box::new(a), Box::new(self.and()?));
        }
        Ok(a)
    }

    fn and(&mut self) -> Result<Node, Error> {
        let mut a = self.binary(0)?;
        while self.eat("&&")? {
            a = Node::And(Box::new(a), Box::new(self.binary(0)?));
        }
        Ok(a)
    }

    /// Equality, then comparisons, then sums, then products.
    fn binary(&mut self, level: usize) -> Result<Node, Error> {
        const LEVELS: [&[(&str, Bin)]; 4] = [
            &[("==", Bin::Eq), ("!=", Bin::Ne)],
            &[
                ("<=", Bin::Le),
                (">=", Bin::Ge),
                ("<", Bin::Lt),
                (">", Bin::Gt),
            ],
            &[("+", Bin::Add), ("-", Bin::Sub)],
            &[("*", Bin::Mul), ("/", Bin::Div), ("%", Bin::Rem)],
        ];
        let Some(ops) = LEVELS.get(level) else {
            return self.unary();
        };
        let mut a = self.binary(level + 1)?;
        'more: loop {
            for (sym, op) in ops.iter() {
                if self.eat(sym)? {
                    a = Node::Bin(*op, Box::new(a), Box::new(self.binary(level + 1)?));
                    continue 'more;
                }
            }
            return Ok(a);
        }
    }

    fn unary(&mut self) -> Result<Node, Error> {
        if self.eat("-")? {
            return Ok(Node::Neg(Box::new(self.unary()?)));
        }
        if self.eat("!")? {
            return Ok(Node::Not(Box::new(self.unary()?)));
        }
        let at = self.at;
        match self.tok {
            Tok::Num(v) => {
                self.next()?;
                Ok(Node::Num(v))
            }
            Tok::Sym("(") => {
                self.next()?;
                let e = self.or()?;
                self.expect(")")?;
                Ok(e)
            }
            Tok::Name(name) => {
                self.next()?;
                if self.tok == Tok::Sym("(") {
                    return self.call(name, at);
                }
                Ok(match name {
                    "pi" => Node::Num(PI),
                    "e" => Node::Num(std::f32::consts::E),
                    "nan" => Node::Num(f32::NAN),
                    "true" => Node::Num(1.0),
                    "false" => Node::Num(0.0),
                    _ => match (self.resolve)(name) {
                        Some(slot) => Node::Slot(slot),
                        None => return err(format!("unknown variable {name}"), at),
                    },
                })
            }
            Tok::End => err("unexpected end", at),
            Tok::Sym(s) => err(format!("unexpected {s}"), at),
        }
    }

    fn call(&mut self, name: &str, at: usize) -> Result<Node, Error> {
        self.expect("(")?;
        let mut args = Vec::new();
        if !self.eat(")")? {
            loop {
                args.push(self.or()?);
                if self.eat(")")? {
                    break;
                }
                self.expect(",")?;
            }
        }
        if name == "if" || name == "ifb" {
            if args.len() < 3 || args.len() % 2 == 0 {
                return err(format!("{name} takes cond, value pairs and an else"), at);
            }
            return Ok(Node::If(args));
        }
        let Some((f, min, max)) = Func::named(name) else {
            return err(format!("unknown function {name}"), at);
        };
        if args.len() < min || args.len() > max.min(u8::MAX as usize) {
            return err(format!("wrong number of arguments to {name}"), at);
        }
        Ok(Node::Call(f, args))
    }
}

/// Folds the constant parts of `n`.
fn fold(n: Node) -> Node {
    let num = |n: &Node| match n {
        Node::Num(v) => Some(*v),
        _ => None,
    };
    let truth = |v: f32| if v != 0.0 { 1.0 } else { 0.0 };
    match n {
        Node::Neg(a) => match fold(*a) {
            Node::Num(v) => Node::Num(-v),
            a => Node::Neg(Box::new(a)),
        },
        Node::Not(a) => match fold(*a) {
            Node::Num(v) => Node::Num(1.0 - truth(v)),
            a => Node::Not(Box::new(a)),
        },
        Node::Bin(op, a, b) => match (fold(*a), fold(*b)) {
            (Node::Num(x), Node::Num(y)) => Node::Num(op.apply(x, y)),
            (a, b) => Node::Bin(op, Box::new(a), Box::new(b)),
        },
        Node::And(a, b) => match fold(*a) {
            Node::Num(0.0) => Node::Num(0.0),
            Node::Num(_) => fold(Node::Not(Box::new(Node::Not(b)))),
            a => Node::And(Box::new(a), Box::new(fold(*b))),
        },
        Node::Or(a, b) => match fold(*a) {
            Node::Num(v) if v != 0.0 => Node::Num(1.0),
            Node::Num(_) => fold(Node::Not(Box::new(Node::Not(b)))),
            a => Node::Or(Box::new(a), Box::new(fold(*b))),
        },
        Node::If(args) => {
            let mut out = Vec::new();
            let mut args = args.into_iter();
            while args.len() > 1 {
                let (c, v) = (fold(args.next().unwrap()), args.next().unwrap());
                match num(&c) {
                    // A false branch is dropped, a true one ends the chain.
                    Some(0.0) => continue,
                    Some(_) if out.is_empty() => return fold(v),
                    Some(_) => {
                        out.push(fold(v));
                        return Node::If(out);
                    }
                    None => out.extend([c, fold(v)]),
                }
            }
            let otherwise = fold(args.next().unwrap());
            if out.is_empty() {
                return otherwise;
            }
            out.push(otherwise);
            Node::If(out)
        }
        Node::Call(f, args) => {
            let args: Vec<Node> = args.into_iter().map(fold).collect();
            match args.iter().map(num).collect::<Option<Vec<f32>>>() {
                Some(values) if !f.varies(values.len()) => Node::Num(f.apply(&values)),
                _ => Node::Call(f, args),
            }
        }
        n => n,
    }
}

struct Emit {
    code: Vec<Op>,
    depth: usize,
    max: usize,
}
impl Emit {
    fn push(&mut self, op: Op, change: isize) {
        self.code.push(op);
        self.depth = self
            .depth
            .checked_add_signed(change)
            .expect("stack underflow");
        self.max = self.max.max(self.depth);
    }
    fn here(&self) -> u32 {
        self.code.len() as u32
    }
    fn patch(&mut self, at: u32) {
        let to = self.here();
        match &mut self.code[at as usize] {
            Op::JumpIfZero(t) | Op::Jump(t) => *t = to,
            _ => unreachable!(),
        }
    }
    fn node(&mut self, n: &Node) {
        match n {
            Node::Num(v) => self.push(Op::Const(*v), 1),
            Node::Slot(s) => self.push(Op::Slot(*s), 1),
            Node::Neg(a) => {
                self.node(a);
                self.push(Op::Neg, 0);
            }
            Node::Not(a) => {
                self.node(a);
                self.push(Op::Not, 0);
            }
            Node::Bin(op, a, b) => {
                self.node(a);
                self.node(b);
                self.push(Op::Bin(*op), -1);
            }
            Node::And(a, b) => {
                self.node(a);
                let skip = self.here();
                self.push(Op::JumpIfZero(0), -1);
                self.node(b);
                self.push(Op::Bool, 0);
                let end = self.here();
                self.push(Op::Jump(0), -1);
                self.patch(skip);
                self.push(Op::Const(0.0), 1);
                self.patch(end);
            }
            Node::Or(a, b) => {
                self.node(a);
                let other = self.here();
                self.push(Op::JumpIfZero(0), -1);
                self.push(Op::Const(1.0), 1);
                let end = self.here();
                self.push(Op::Jump(0), -1);
                self.patch(other);
                self.node(b);
                self.push(Op::Bool, 0);
                self.patch(end);
            }
            Node::If(args) => {
                let mut ends = Vec::new();
                for pair in args.chunks_exact(2) {
                    self.node(&pair[0]);
                    let next = self.here();
                    self.push(Op::JumpIfZero(0), -1);
                    self.node(&pair[1]);
                    ends.push(self.here());
                    self.push(Op::Jump(0), -1);
                    self.patch(next);
                }
                self.node(args.last().unwrap());
                for end in ends {
                    self.patch(end);
                }
            }
            Node::Call(f, args) => {
                for a in args {
                    self.node(a);
                }
                self.push(Op::Call(*f, args.len() as u8), 1 - args.len() as isize);
            }
        }
    }
}

/// Compiles `src`. `resolve` gives the slot of a variable name, None for one that doesn't
/// exist.
pub fn compile(src: &str, resolve: &mut dyn FnMut(&str) -> Option<u32>) -> Result<Expr, Error> {
    let mut p = Parser {
        src,
        pos: 0,
        tok: Tok::End,
        at: 0,
        resolve,
    };
    p.next()?;
    let tree = p.or()?;
    if p.tok != Tok::End {
        return err("unexpected text after the expression", p.at);
    }
    let tree = fold(tree);
    let mut emit = Emit {
        code: Vec::new(),
        depth: 0,
        max: 0,
    };
    emit.node(&tree);
    if emit.max > STACK {
        return err("expression too deep", 0);
    }
    Ok(Expr { code: emit.code })
}

impl Expr {
    /// The value with the variables in `slots`.
    pub fn eval(&self, slots: &[f32]) -> f32 {
        let mut stack = [0.0f32; STACK];
        let mut sp = 0;
        let mut pc = 0;
        while let Some(op) = self.code.get(pc) {
            pc += 1;
            match *op {
                Op::Const(v) => {
                    stack[sp] = v;
                    sp += 1;
                }
                Op::Slot(s) => {
                    stack[sp] = slots.get(s as usize).copied().unwrap_or(0.0);
                    sp += 1;
                }
                Op::Neg => stack[sp - 1] = -stack[sp - 1],
                Op::Not => stack[sp - 1] = if stack[sp - 1] == 0.0 { 1.0 } else { 0.0 },
                Op::Bool => stack[sp - 1] = if stack[sp - 1] != 0.0 { 1.0 } else { 0.0 },
                Op::Bin(op) => {
                    sp -= 1;
                    stack[sp - 1] = op.apply(stack[sp - 1], stack[sp]);
                }
                Op::Call(f, n) => {
                    let n = n as usize;
                    let v = f.apply(&stack[sp - n..sp]);
                    sp -= n;
                    stack[sp] = v;
                    sp += 1;
                }
                Op::JumpIfZero(to) => {
                    sp -= 1;
                    if stack[sp] == 0.0 {
                        pc = to as usize;
                    }
                }
                Op::Jump(to) => pc = to as usize,
            }
        }
        stack[0]
    }

    /// The value, when it doesn't depend on any variable or on chance.
    pub fn constant(&self) -> Option<f32> {
        match self.code.as_slice() {
            [Op::Const(v)] => Some(*v),
            _ => None,
        }
    }

    /// The slots it reads.
    pub fn slots(&self) -> impl Iterator<Item = u32> + '_ {
        self.code.iter().filter_map(|op| match op {
            Op::Slot(s) => Some(*s),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Slots: `a` 0, `b` 1, `var.n` 2, `head.rx` 3.
    const NAMES: [&str; 4] = ["a", "b", "var.n", "head.rx"];
    fn names(n: &str) -> Option<u32> {
        NAMES.iter().position(|x| *x == n).map(|i| i as u32)
    }
    fn expr(src: &str) -> Expr {
        compile(src, &mut names).unwrap_or_else(|e| panic!("{src}: {e}"))
    }
    fn eval(src: &str, slots: &[f32]) -> f32 {
        expr(src).eval(slots)
    }
    fn value(src: &str) -> f32 {
        eval(src, &[])
    }
    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn precedence_and_booleans() {
        assert_eq!(value("1 + 2 * 3"), 7.0);
        assert_eq!(value("(1 + 2) * 3"), 9.0);
        assert_eq!(value("-2 * 3 + 10 % 4"), -4.0);
        assert_eq!(value("1 < 2 == 1"), 1.0);
        assert_eq!(value("1 + 1 == 2 && !(3 > 4) || false"), 1.0);
        assert_eq!(value("2 && 0.5"), 1.0);
        assert_eq!(value("1 - -1"), 2.0);
        assert_eq!(eval("a * 2 + b", &[3.0, 1.0]), 7.0);
        assert_eq!(eval("head.rx + var.n", &[0.0, 0.0, 2.0, 0.5]), 2.5);
    }

    #[test]
    fn branches_not_taken_are_not_evaluated() {
        // A taken `random()` would advance the shared sequence.
        let first = expr("random()").eval(&[]);
        let untaken = expr("if(a, random(), b)");
        assert_eq!(untaken.eval(&[0.0, 5.0]), 5.0);
        assert_ne!(expr("random()").eval(&[]), first);
        // Each condition in order, then the else.
        let chain = expr("if(a > 2, 10, a > 1, 20, 30)");
        assert_eq!(
            [3.0, 1.5, 0.0].map(|a| chain.eval(&[a])),
            [10.0, 20.0, 30.0]
        );
        // 0 && x never reads x; 1 || x neither.
        assert_eq!(eval("a && b / 0 > 1", &[0.0, 1.0]), 0.0);
        assert_eq!(eval("a || b", &[1.0, 0.0]), 1.0);
        assert_eq!(eval("a || b", &[0.0, 7.0]), 1.0);
    }

    #[test]
    fn functions() {
        let pi = std::f32::consts::PI;
        assert_eq!(value("min(4, 2, 8, 3)"), 2.0);
        assert_eq!(value("max(4, 2, 8, 3)"), 8.0);
        assert_eq!(value("clamp(5, 0, 2)"), 2.0);
        assert_eq!(value("-7 % 3"), -1.0);
        assert_eq!(value("fmod(-7, 3)"), 2.0);
        assert_eq!(value("frac(-1.25)"), 0.75);
        assert_eq!(value("round(2.5) + round(-2.5)"), 1.0);
        assert_eq!(value("signum(-3) + signum(0)"), -1.0);
        assert!(close(value("torad(180)"), pi));
        assert!(close(value("todeg(pi / 2)"), 90.0));
        assert_eq!(value("lerp(0.25, 4, 8)"), 5.0);
        assert_eq!(value("between(2, 1, 3) + between(4, 1, 3)"), 1.0);
        assert_eq!(value("equals(1, 1.05, 0.1) + in(3, 1, 2, 3)"), 2.0);
        assert_eq!(value("print(1, 20, 6)"), 6.0);
        assert_eq!(value("random(4)"), value("random(4)"));
        assert!((0.0..1.0).contains(&value("random(4)")));
        assert_eq!(value("catch(0 / 0, 3)"), 3.0);
        assert_eq!(value("catch(2, 3, 1)"), 2.0);
        // EMF's documented examples.
        assert_eq!(value("wrapdeg(370)"), 10.0);
        assert_eq!(value("wrapdeg(350)"), -10.0);
        assert_eq!(value("degdiff(10, 350)"), -20.0);
        assert_eq!(value("degdiff(10, 370)"), 0.0);
    }

    #[test]
    fn keyframes_and_curves() {
        let at = |k: f32, f: &str| eval(&format!("{f}(a, 5, 10, 20)"), &[k]);
        assert_eq!(at(0.0, "keyframe"), 5.0);
        assert_eq!(at(1.0, "keyframe"), 10.0);
        assert_eq!(at(1.5, "keyframe"), 15.0);
        assert_eq!(at(7.0, "keyframe"), 20.0);
        assert_eq!(at(-1.5, "keyframe"), 15.0);
        // The loop goes from the last frame back to the first.
        assert_eq!(at(2.5, "keyframeloop"), 12.5);
        assert_eq!(at(3.5, "keyframeloop"), 7.5);
        // Ends of every curve are its two values.
        for f in [
            "easeinoutexpo",
            "easeinquad",
            "easeoutbounce",
            "easeinoutelastic",
            "easeinoutback",
            "easeincirc",
            "easeoutsine",
            "easeinoutquint",
        ] {
            assert!(close(eval(&format!("{f}(a, 2, 6)"), &[0.0]), 2.0), "{f}");
            assert!(close(eval(&format!("{f}(a, 2, 6)"), &[1.0]), 6.0), "{f}");
        }
        assert_eq!(value("easeinquad(0.5, 0, 8)"), 2.0);
        assert_eq!(value("easeoutquad(0.5, 0, 8)"), 6.0);
        assert_eq!(value("quadbezier(0.5, 0, 4, 2)"), 2.0);
        assert_eq!(value("cubicbezier(0.5, 0, 8, 0, 8)"), 4.0);
        assert_eq!(value("catmullrom(0, 0, 1, 2, 3)"), 1.0);
        assert_eq!(value("catmullrom(1, 0, 1, 2, 3)"), 2.0);
        assert_eq!(value("hermite(1, 3, 7, 0, 0)"), 7.0);
    }

    #[test]
    fn constants_fold() {
        let pi = std::f32::consts::PI;
        assert!(close(expr("torad(180)").constant().unwrap(), pi));
        assert_eq!(expr("if(true, 2, a)").constant(), Some(2.0));
        assert_eq!(expr("if(false, a, 1 > 0, 3, b)").constant(), Some(3.0));
        assert_eq!(expr("0 && a").constant(), Some(0.0));
        assert_eq!(expr("random()").constant(), None);
        assert_eq!(expr("a + 1").constant(), None);
        // A false branch goes; the variable ones stay.
        let partly = expr("if(false, a, b > 1, b, 4)");
        assert_eq!(partly.slots().collect::<Vec<_>>(), [1, 1]);
        assert_eq!(partly.eval(&[9.0, 2.0]), 2.0);
        assert_eq!(partly.eval(&[9.0, 0.0]), 4.0);
    }

    #[test]
    fn errors_say_where() {
        let e = |src: &str| compile(src, &mut names).unwrap_err();
        assert_eq!(e("a + nope").offset, 4);
        assert!(e("a + nope").message.contains("nope"));
        assert_eq!(e("sinh(a)").offset, 0);
        assert!(e("clamp(1, 2)").message.contains("arguments"));
        assert!(e("if(a, b)").message.contains("if"));
        assert_eq!(e("(a + 1").offset, 6);
        assert_eq!(e("a + 1)").offset, 5);
        assert_eq!(e("a # b").offset, 2);
        let deep = format!("{}a{}", "(a + ".repeat(70), ")".repeat(70));
        assert!(e(&deep).message.contains("deep"));
    }
}
