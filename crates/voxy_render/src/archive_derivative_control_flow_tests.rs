//! The backend currently accepts fragment derivatives in divergent control flow;
//! validation alone therefore does not guard the pixel-dependent sidewalk mask.
use super::archive_shader_source;

fn has_derivative(block: &naga::Block, function: &naga::Function) -> bool {
    block.iter().any(|statement| match statement {
        naga::Statement::Emit(range) => range
            .clone()
            .any(|h| matches!(function.expressions[h], naga::Expression::Derivative { .. })),
        naga::Statement::Block(b) => has_derivative(b, function),
        naga::Statement::If { accept, reject, .. } => {
            has_derivative(accept, function) || has_derivative(reject, function)
        }
        naga::Statement::Switch { cases, .. } => {
            cases.iter().any(|c| has_derivative(&c.body, function))
        }
        naga::Statement::Loop {
            body, continuing, ..
        } => has_derivative(body, function) || has_derivative(continuing, function),
        _ => false,
    })
}
fn inspect_mask(
    block: &naga::Block,
    function: &naga::Function,
    mask: naga::Handle<naga::Expression>,
) -> usize {
    let mut found = 0;
    for statement in block.iter() {
        match statement {
            naga::Statement::If {
                condition,
                accept,
                reject,
            } => {
                if matches!(function.expressions[*condition], naga::Expression::Binary { op: naga::BinaryOperator::Greater, left, .. } if left == mask)
                {
                    found += 1;
                    assert!(
                        !has_derivative(accept, function) && !has_derivative(reject, function),
                        "screen footprint is indeterminate inside a pixel-dependent slab mask"
                    );
                }
                found +=
                    inspect_mask(accept, function, mask) + inspect_mask(reject, function, mask);
            }
            naga::Statement::Block(b) => found += inspect_mask(b, function, mask),
            naga::Statement::Switch { cases, .. } => {
                for c in cases {
                    found += inspect_mask(&c.body, function, mask);
                }
            }
            naga::Statement::Loop {
                body, continuing, ..
            } => {
                found +=
                    inspect_mask(body, function, mask) + inspect_mask(continuing, function, mask)
            }
            _ => {}
        }
    }
    found
}
#[test]
fn sidewalk_filter_derivatives_are_outside_pixel_dependent_mask() {
    let module = naga::front::wgsl::parse_str(&archive_shader_source()).unwrap();
    let (_, function) = module
        .functions
        .iter()
        .find(|(_, f)| f.name.as_deref() == Some("original_surface"))
        .unwrap();
    let mask = *function
        .named_expressions
        .iter()
        .find(|(_, name)| name.as_str() == "side_slab")
        .unwrap()
        .0;
    assert_eq!(
        inspect_mask(&function.body, function, mask),
        1,
        "sidewalk coverage guard must be inspected in the actual production shader"
    );
}

#[test]
fn material_footprints_use_explicit_fine_quad_derivatives() {
    let module = naga::front::wgsl::parse_str(&archive_shader_source()).unwrap();
    let mut checked = 0;
    for (_, function) in module.functions.iter().filter(|(_, f)| {
        matches!(
            f.name.as_deref(),
            Some(
                "original_surface"
                    | "original_height"
                    | "original_bump"
                    | "archive_surface"
                    | "wet_layer"
            )
        )
    }) {
        for (_, expression) in function.expressions.iter() {
            if let naga::Expression::Derivative { ctrl, .. } = expression {
                assert_eq!(
                    *ctrl,
                    naga::DerivativeControl::Fine,
                    "{} must not depend on the backend choosing coarse or fine derivatives",
                    function.name.as_deref().unwrap()
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 10,
        "actual material footprint expressions must be inspected"
    );
}
