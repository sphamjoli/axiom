//! Which parameters are the exit probabilities of a two-state chain.

mod common;

use common::{TestError, chain, fixture};

/// The names of the exit parameters, `a` then `b`, if the chain has them.
fn exits(source: &str) -> Result<Option<(String, String)>, TestError> {
    let model = chain(source)?;
    Ok(model
        .two_state_exits()
        .map(|(a, b)| (a.name.clone(), b.name.clone())))
}

#[test]
fn links_param_exits_are_a_then_b() -> Result<(), TestError> {
    assert_eq!(
        exits(&fixture("links-param.axm")?)?,
        Some((String::from("a"), String::from("b")))
    );
    Ok(())
}

/// The exits follow the rows, not the declaration order.
#[test]
fn exits_follow_the_rows_not_the_declarations() -> Result<(), TestError> {
    let swapped = "param a in ]0, 1[\nparam b in ]0, 1[\nchain c {\n  states s, t\n  from s: s 1 - b, t b\n  from t: s a, t 1 - a\n}\n";
    assert_eq!(
        exits(swapped)?,
        Some((String::from("b"), String::from("a")))
    );
    Ok(())
}

/// An exit that is a multiple of a parameter, an offset parameter, a fixed
/// exit, one parameter for both exits or a third state is not the
/// supported shape.
#[test]
fn other_parametric_chains_have_no_exits() -> Result<(), TestError> {
    let models = [
        "param a in ]0, 1[\nparam b in ]0, 1[\nchain c {\n  states s, t\n  from s: s 1 - 1/2*a, t 1/2*a\n  from t: s b, t 1 - b\n}\n",
        "param a in ]0, 1/2[\nparam b in ]0, 1[\nchain c {\n  states s, t\n  from s: s 9/10 - a, t 1/10 + a\n  from t: s b, t 1 - b\n}\n",
        "param a in ]0, 1[\nchain c {\n  states s, t\n  from s: s 1 - a, t a\n  from t: s 1/2, t 1/2\n}\n",
        "param a in ]0, 1[\nchain c {\n  states s, t\n  from s: s 1 - a, t a\n  from t: s a, t 1 - a\n}\n",
        "param a in ]0, 1[\nparam b in ]0, 1[\nchain c {\n  states s, t, u\n  from s: s 1 - a, t a\n  from t: s b, t 1 - b\n  from u: u 1\n}\n",
    ];
    for model in models {
        assert_eq!(exits(model)?, None, "{model}");
    }
    Ok(())
}
