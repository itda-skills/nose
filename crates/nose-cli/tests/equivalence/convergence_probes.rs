use super::*;

/// Exploratory probe (research): candidate SOUND algebraic/boolean equivalences that
/// stress phase-ordering — does a single bottom-up `mk` pass reach the canonical form,
/// or would a fixpoint/saturation be needed? Not an assertion; a frontier map.
/// Run: cargo test convergence_probe5 -- --nocapture
#[test]
fn convergence_probe5() {
    let i = Interner::new();
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        // Distribution in the EXPANSION direction (current code only FACTORS).
        (
            "distribute-expand",
            "def f(a,b,c):\n    return c*(a+b)\n",
            Lang::Python,
            "def g(a,b,c):\n    return c*a+c*b\n",
            Lang::Python,
        ),
        // Factor where the shared multiplicand is on the LEFT of one product.
        (
            "factor-left-shared",
            "def f(a,b,c):\n    return c*a+b*c\n",
            Lang::Python,
            "def g(a,b,c):\n    return (a+b)*c\n",
            Lang::Python,
        ),
        // De Morgan composed with comparison-direction: needs algebra THEN compare-canon.
        (
            "demorgan+cmp",
            "def f(a,b):\n    return not (a>b or a==b)\n",
            Lang::Python,
            "def g(a,b):\n    return a<b\n",
            Lang::Python,
        ),
        // Nested distribution requiring re-canonicalization of a synthesized node.
        (
            "distribute-3term",
            "def f(a,b,d,c):\n    return a*c+b*c+d*c\n",
            Lang::Python,
            "def g(a,b,d,c):\n    return (a+b+d)*c\n",
            Lang::Python,
        ),
        // Distribution feeding AC sort: (a+b)*c + e  vs  c*b + c*a + e
        (
            "distribute-then-ac",
            "def f(a,b,c,e):\n    return c*b+c*a+e\n",
            Lang::Python,
            "def g(a,b,c,e):\n    return (a+b)*c+e\n",
            Lang::Python,
        ),
        // Double negation pushed through a comparison then re-canon.
        (
            "not-not-cmp",
            "def f(a,b):\n    return not (not (a>b))\n",
            Lang::Python,
            "def g(a,b):\n    return b<a\n",
            Lang::Python,
        ),
        // Negation distributed then factored back.
        (
            "neg-distribute-factor",
            "def f(a,b,c):\n    return -(a*c+b*c)\n",
            Lang::Python,
            "def g(a,b,c):\n    return -((a+b)*c)\n",
            Lang::Python,
        ),
        // Decompose the demorgan+cmp gap:
        // (a) lattice fact alone: (a<=b) ∧ (a!=b) ≡ a<b
        (
            "lattice-le-ne",
            "def f(a,b):\n    return a<=b and a!=b\n",
            Lang::Python,
            "def g(a,b):\n    return a<b\n",
            Lang::Python,
        ),
        // (b) De Morgan over OR alone in the value graph
        (
            "demorgan-or",
            "def f(a,b,c):\n    return not (a<b or c<b)\n",
            Lang::Python,
            "def g(a,b,c):\n    return a>=b and c>=b\n",
            Lang::Python,
        ),
        // (c) De Morgan over AND alone
        (
            "demorgan-and",
            "def f(a,b,c):\n    return not (a<b and c<b)\n",
            Lang::Python,
            "def g(a,b,c):\n    return a>=b or c>=b\n",
            Lang::Python,
        ),
    ];
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!("probe5: {}/{} converge", pairs.len() - gaps, pairs.len());
}

/// Convergence probe (research): diverse genuinely-equivalent pairs that a strong IL
/// SHOULD converge. Prints which converge vs not — the non-converging ones are IL gaps to
/// close. Not an assertion (a map of the frontier). Run: cargo test convergence_probe -- --nocapture
#[test]
fn convergence_probe() {
    let pairs: &[(&str, &str, &str)] = &[
        ("nested-if vs conjunction",
         "def f(a,b,c):\n    if a>0:\n        if b>0:\n            return c+1\n    return c+2\n",
         "def g(a,b,c):\n    if a>0 and b>0:\n        return c+1\n    return c+2\n"),
        ("else-after-return vs guard",
         "def f(a,b):\n    if a>0:\n        return b+1\n    else:\n        return b+2\n",
         "def g(a,b):\n    if a>0:\n        return b+1\n    return b+2\n"),
        ("ternary vs if-assign",
         "def f(a,b):\n    x = b+1 if a>0 else b+2\n    return x*3\n",
         "def g(a,b):\n    if a>0:\n        x = b+1\n    else:\n        x = b+2\n    return x*3\n"),
        ("filter fusion",
         "def f(xs):\n    return [x for x in xs if x>0 if x<10]\n",
         "def g(xs):\n    return [x for x in [y for y in xs if y>0] if x<10]\n"),
        ("map-filter (filter then map)",
         "def f(xs):\n    return [h(x) for x in xs if x>0]\n",
         "def g(xs):\n    return [h(y) for y in [x for x in xs if x>0]]\n"),
        ("fold-map fusion (sum of squares)",
         "def f(xs):\n    return sum(x*x for x in xs)\n",
         "def g(xs):\n    t = 0\n    for x in xs:\n        t += x*x\n    return t\n"),
        ("De Morgan",
         "def f(a,b):\n    return not (a>0 and b>0)\n",
         "def g(a,b):\n    return a<=0 or b<=0\n"),
        ("comparison swap inside",
         "def f(a,b):\n    return (a>b) and (b>0)\n",
         "def g(a,b):\n    return (0<b) and (b<a)\n"),
        ("double map then sum",
         "def f(xs):\n    return sum(g(f(x)) for x in xs)\n",
         "def g2(xs):\n    return sum(g(y) for y in [f(x) for x in xs])\n"),
        ("while vs for-range sum",
         "def f(xs):\n    t=0\n    for i in range(len(xs)):\n        t+=xs[i]\n    return t\n",
         "def g(xs):\n    t=0\n    i=0\n    while i<len(xs):\n        t+=xs[i]\n        i+=1\n    return t\n"),
    ];
    let i = Interner::new();
    let mut gaps = 0;
    for (name, a, b) in pairs {
        let eq = value_fp(&i, a, Lang::Python) == value_fp(&i, b, Lang::Python);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!(
        "convergence probe: {}/{} converge",
        pairs.len() - gaps,
        pairs.len()
    );
}

/// Cross-language + more-construct convergence probe (research): the SAME algorithm in
/// different languages / forms should converge to one fingerprint. Maps the frontier.
#[test]
fn convergence_probe_xlang() {
    // (name, srcA, langA, srcB, langB)
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        ("sum-loop Py vs JS-reduce",
         "def f(xs):\n    t=0\n    for x in xs:\n        t+=x\n    return t\n", Lang::Python,
         "function f(xs){ return xs.reduce((a,x)=>a+x, 0); }", Lang::JavaScript),
        ("sum-loop Py vs Go",
         "def f(xs):\n    t=0\n    for x in xs:\n        t+=x\n    return t\n", Lang::Python,
         "package p\nfunc f(xs []int) int {\n\tt := 0\n\tfor _, x := range xs {\n\t\tt += x\n\t}\n\treturn t\n}\n", Lang::Go),
        ("sum-loop Py vs Rust-fold",
         "def f(xs):\n    t=0\n    for x in xs:\n        t+=x\n    return t\n", Lang::Python,
         "fn f(xs: &[i64]) -> i64 { xs.iter().fold(0, |a, x| a + x) }", Lang::Rust),
        ("map Py vs JS",
         "def f(xs):\n    return [x*x for x in xs]\n", Lang::Python,
         "function f(xs){ return xs.map(x => x*x); }", Lang::JavaScript),
        ("guard Py vs Go",
         "def f(a,b):\n    if a>0:\n        return b+1\n    return b+2\n", Lang::Python,
         "package p\nfunc f(a,b int) int {\n\tif a>0 {\n\t\treturn b+1\n\t}\n\treturn b+2\n}\n", Lang::Go),
        ("x*2 vs x+x", "def f(x):\n    return x*2\n", Lang::Python, "def g(x):\n    return x+x\n", Lang::Python),
        ("abs idioms Py", "def f(x: int):\n    return x if x>=0 else -x\n", Lang::Python, "def g(x: int):\n    return abs(x)\n", Lang::Python),
        ("compound assign", "def f(a,b):\n    a += b\n    a *= 2\n    return a\n", Lang::Python, "def g(a,b):\n    return (a+b)*2\n", Lang::Python),
        ("min idioms", "def f(a,b):\n    return a if a<b else b\n", Lang::Python, "def g(a,b):\n    return min(a,b)\n", Lang::Python),
        ("count loop vs sum-1", "def f(xs):\n    c=0\n    for x in xs:\n        if x>0:\n            c+=1\n    return c\n", Lang::Python, "def g(xs):\n    return sum(1 for x in xs if x>0)\n", Lang::Python),
    ];
    let i = Interner::new();
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!(
        "xlang probe: {}/{} converge",
        pairs.len() - gaps,
        pairs.len()
    );
}

/// More-construct convergence probe (research, batch 2): widen the frontier map.
#[test]
fn convergence_probe2() {
    let i = Interner::new();
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        ("chained compare", "def f(a,b,c):\n    return a<b<c\n", Lang::Python, "def g(a,b,c):\n    return a<b and b<c\n", Lang::Python),
        ("aug-sub vs assign", "def f(a,b):\n    a -= b\n    return a\n", Lang::Python, "def g(a,b):\n    return a-b\n", Lang::Python),
        ("not-eq vs !=", "def f(a,b):\n    return not (a==b)\n", Lang::Python, "def g(a,b):\n    return a!=b\n", Lang::Python),
        ("double not", "def f(a,b):\n    return not (not (a<b))\n", Lang::Python, "def g(a,b):\n    return a<b\n", Lang::Python),
        ("or-default vs ternary", "def f(a,b):\n    return a if a else b\n", Lang::Python, "def g(a,b):\n    return a or b\n", Lang::Python),
        ("nested ternary vs elif", "def f(a):\n    return 1 if a>0 else (2 if a<0 else 3)\n", Lang::Python, "def g(a):\n    if a>0:\n        return 1\n    elif a<0:\n        return 2\n    return 3\n", Lang::Python),
        ("product loop vs reduce", "def f(xs):\n    p=1\n    for x in xs:\n        p*=x\n    return p\n", Lang::Python, "def g(xs):\n    r=1\n    for x in xs:\n        r=r*x\n    return r\n", Lang::Python),
        ("max loop vs max()", "def f(xs):\n    m=xs[0]\n    for x in xs:\n        if x>m:\n            m=x\n    return m\n", Lang::Python, "def g(xs):\n    return max(xs)\n", Lang::Python),
        ("filter-map JS vs Py", "function f(xs){ return xs.filter(x=>x>0).map(x=>h(x)); }", Lang::JavaScript, "def g(xs):\n    return [h(x) for x in xs if x>0]\n", Lang::Python),
        ("early-continue vs filter", "def f(xs):\n    t=0\n    for x in xs:\n        if x<=0:\n            continue\n        t+=x\n    return t\n", Lang::Python, "def g(xs):\n    return sum(x for x in xs if x>0)\n", Lang::Python),
        ("swap temps", "def f(a,b):\n    t=a\n    a=b\n    b=t\n    return a-b\n", Lang::Python, "def g(a,b):\n    return b-a\n", Lang::Python),
        ("redundant paren-group", "def f(a,b,c):\n    return (a+b)+c\n", Lang::Python, "def g(a,b,c):\n    return a+(b+c)\n", Lang::Python),
    ];
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!("probe2: {}/{} converge", pairs.len() - gaps, pairs.len());
}

/// Convergence probe batch 3 (research): slices, enumerate, dict, recursion, more xlang.
#[test]
fn convergence_probe3() {
    let i = Interner::new();
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        ("enumerate vs range-index", "def f(xs):\n    t=0\n    for i,x in enumerate(xs):\n        t+=i*x\n    return t\n", Lang::Python, "def g(xs):\n    t=0\n    for i in range(len(xs)):\n        t+=i*xs[i]\n    return t\n", Lang::Python),
        ("dict-comp vs loop", "def f(xs):\n    return {x: x*x for x in xs}\n", Lang::Python, "def g(xs):\n    d={}\n    for x in xs:\n        d[x]=x*x\n    return d\n", Lang::Python),
        ("any vs or-loop", "def f(xs):\n    return any(x>0 for x in xs)\n", Lang::Python, "def g(xs):\n    for x in xs:\n        if x>0:\n            return True\n    return False\n", Lang::Python),
        ("all vs and-loop", "def f(xs):\n    return all(x>0 for x in xs)\n", Lang::Python, "def g(xs):\n    for x in xs:\n        if not (x>0):\n            return False\n    return True\n", Lang::Python),
        ("reversed-compare", "def f(a,b):\n    return a>=b\n", Lang::Python, "def g(a,b):\n    return b<=a\n", Lang::Python),
        ("neg distribute", "def f(a,b):\n    return -(a+b)\n", Lang::Python, "def g(a,b):\n    return -a-b\n", Lang::Python),
        ("mul-add factor", "def f(a,b,c):\n    return a*c+b*c\n", Lang::Python, "def g(a,b,c):\n    return (a+b)*c\n", Lang::Python),
        ("string concat order", "def f(a,b):\n    return a+b+a\n", Lang::Python, "def g(a,b):\n    return a+(b+a)\n", Lang::Python),
        ("Go for-i vs Py range", "package p\nfunc f(xs []int) int {\n\tt:=0\n\tfor i:=0;i<len(xs);i++{\n\t\tt+=xs[i]\n\t}\n\treturn t\n}\n", Lang::Go, "def g(xs):\n    t=0\n    for i in range(len(xs)):\n        t+=xs[i]\n    return t\n", Lang::Python),
        ("Rust map vs Py comp", "fn f(xs:&[i64])->Vec<i64>{ xs.iter().map(|x| x*x).collect() }", Lang::Rust, "def g(xs):\n    return [x*x for x in xs]\n", Lang::Python),
        ("compound vs explicit", "def f(a):\n    a //= 2\n    return a\n", Lang::Python, "def g(a):\n    return a // 2\n", Lang::Python),
        ("nested-neg", "def f(x):\n    return -(-(-x))\n", Lang::Python, "def g(x):\n    return -x\n", Lang::Python),
    ];
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!("probe3: {}/{} converge", pairs.len() - gaps, pairs.len());
}

/// Convergence probe batch 4 (research): candidate Type-4 equivalences to scope which to
/// close next — negative indexing, count-of-filter, reduce-lambda selection, more idioms.
#[test]
fn convergence_probe4() {
    let i = Interner::new();
    let pairs: &[(&str, &str, Lang, &str, Lang)] = &[
        (
            "neg-index last",
            "def f(s):\n    return s[len(s)-1]\n",
            Lang::Python,
            "def g(s):\n    return s[-1]\n",
            Lang::Python,
        ),
        (
            "neg-index k",
            "def f(s):\n    return s[len(s)-2]\n",
            Lang::Python,
            "def g(s):\n    return s[-2]\n",
            Lang::Python,
        ),
        (
            "len-count vs sum-1",
            "def f(xs):\n    return len([x for x in xs if x>0])\n",
            Lang::Python,
            "def g(xs):\n    return sum(1 for x in xs if x>0)\n",
            Lang::Python,
        ),
        (
            "reduce-lambda max vs max()",
            "def f(xs):\n    return reduce(lambda a,b: a if a>b else b, xs)\n",
            Lang::Python,
            "def g(xs):\n    return max(xs)\n",
            Lang::Python,
        ),
        (
            "reduce-lambda min vs min()",
            "def f(xs):\n    return reduce(lambda a,b: a if a<b else b, xs)\n",
            Lang::Python,
            "def g(xs):\n    return min(xs)\n",
            Lang::Python,
        ),
        (
            "not-in vs not(in)",
            "def f(a,b):\n    return a not in b\n",
            Lang::Python,
            "def g(a,b):\n    return not (a in b)\n",
            Lang::Python,
        ),
        (
            "filter Py vs JS",
            "def f(xs):\n    return [x for x in xs if x>0]\n",
            Lang::Python,
            "function g(xs){ return xs.filter(x=>x>0); }",
            Lang::JavaScript,
        ),
        (
            "sum-filter Py vs Rust",
            "def f(xs):\n    return sum(x for x in xs if x>0)\n",
            Lang::Python,
            "fn g(xs:&[i64])->i64{ xs.iter().filter(|x| **x>0).sum() }",
            Lang::Rust,
        ),
    ];
    let mut gaps = 0;
    for (name, a, la, b, lb) in pairs {
        let eq = value_fp(&i, a, *la) == value_fp(&i, b, *lb);
        if !eq {
            gaps += 1;
        }
        eprintln!("  [{}] {}", if eq { "CONVERGE" } else { "  GAP   " }, name);
    }
    eprintln!("probe4: {}/{} converge", pairs.len() - gaps, pairs.len());
}
