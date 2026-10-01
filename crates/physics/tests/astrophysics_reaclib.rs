use physics::astrophysics_reaclib::{Error, parse};
const DATA: &str = include_str!("data/triple_alpha_fy05.reaclib");
#[test]
fn actual_fy05_records_and_additive_rates() {
    let rate = parse(DATA, 2e8, 2e8, 3).unwrap();
    assert_eq!(rate.reactants, ["he4", "he4", "he4"]);
    assert_eq!(rate.products, ["c12"]);
    assert_eq!(rate.q_mev, 7.275);
    assert!(!rate.reverse);
    assert_eq!(rate.fit.sets.len(), 3);
    assert_eq!(
        rate.fit.sets[0],
        [
            -11.7884, -1.02446, -23.57, 20.4886, -12.9882, -20.0, -2.16667
        ]
    );
    let t: f64 = 0.2;
    let expected: f64 = rate
        .fit
        .sets
        .iter()
        .map(|a| {
            (a[0]
                + a[1] / t
                + a[2] / t.cbrt()
                + a[3] * t.cbrt()
                + a[4] * t
                + a[5] * t.powf(5.0 / 3.0)
                + a[6] * t.ln())
            .exp()
        })
        .sum();
    assert!((rate.fit.evaluate(2e8).unwrap() / expected - 1.0).abs() < 1e-13);
}
#[test]
fn malformed_mixed_weak_budget_and_domain_are_rejected() {
    assert_eq!(parse(DATA, 2e8, 2e8, 2), Err(Error::BudgetExceeded));
    assert_eq!(parse(DATA, 0.0, 2e8, 3), Err(Error::InvalidDomain));
    assert_eq!(
        parse(&DATA.replace("fy05r", "fy05w"), 2e8, 2e8, 3),
        Err(Error::WeakReaction)
    );
    assert_eq!(
        parse(&DATA.replacen("8\n", "10\n", 1), 2e8, 2e8, 3),
        Err(Error::UnsupportedChapter)
    );
    assert!(parse(&DATA[..DATA.len() - 70], 2e8, 2e8, 3).is_err());
    assert_eq!(
        parse(&DATA.replacen("he4", "he3", 1), 2e8, 2e8, 3),
        Err(Error::MixedReaction)
    );
    assert_eq!(parse("8\n💫", 2e8, 2e8, 3), Err(Error::InvalidFormat));
}

#[test]
fn mapping_species_order_and_binding_are_checked() {
    use physics::{
        astrophysics_nuclear::{Network, Nucleus},
        astrophysics_reaclib::MEV_JOULES,
    };
    let rate = parse(DATA, 2e8, 2e8, 3).unwrap();
    let mut network = Network {
        nuclei: vec![
            Nucleus {
                mass_number: 12,
                charge: 6,
                binding_energy: rate.q_mev * MEV_JOULES,
            },
            Nucleus {
                mass_number: 4,
                charge: 2,
                binding_energy: 0.0,
            },
        ],
        reactions: vec![],
    };
    let reaction = rate
        .reaction(&network, &["c12", "he4"], 0.0, 1e-10)
        .unwrap();
    assert_eq!(reaction.reactants, [0, 3]);
    assert_eq!(reaction.products, [1, 0]);
    assert!(network.reactions.is_empty());
    assert_eq!(
        rate.reaction(&network, &["c12", "he3"], 0.0, 1e-10),
        Err(Error::UnknownSpecies)
    );
    assert_eq!(
        rate.reaction(&network, &["he4", "he4"], 0.0, 1e-10),
        Err(Error::InvalidNetwork)
    );
    network.nuclei[0].binding_energy *= 1.01;
    assert_eq!(
        rate.reaction(&network, &["c12", "he4"], 0.0, 1e-10),
        Err(Error::BindingMismatch)
    );
    network.nuclei[0].charge = 5;
    assert_eq!(
        rate.reaction(&network, &["c12", "he4"], 0.0, 1.0),
        Err(Error::InvalidNetwork)
    );
}
