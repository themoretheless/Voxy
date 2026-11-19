//! Trial-step rejections that the inertial solvers recover from by refining time
//! or quadrature.
//!
//! The public API keeps `&'static str` errors (`docs/PLAN.md` R2). This enum is
//! the only source of these messages and the retry policies match on it, so a
//! producer and a policy cannot drift apart through a mistyped string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rejection {
    ClosedSurfaceGap,
    ContactNonlinear,
    ContactLineSearch,
    ContactQuadrature,
    ContactWork,
    ContactPathQuadrature,
    MaterialPathQuadrature,
    MidpointWork,
    ViscoelasticRelaxation,
    ViscoelasticHeat,
    SupportMotionDifference,
    SupportLineSearch,
    SupportSearchDirection,
    SupportNonlinear,
    SupportWork,
    SupportPathCrossing,
    FiniteDeformationSupportWork,
    FiniteDeformationEnergy,
    TetrahedralPathCollapse,
    TissueGapPathCrossing,
}

impl Rejection {
    const ALL: [Self; 20] = [
        Self::ClosedSurfaceGap,
        Self::ContactNonlinear,
        Self::ContactLineSearch,
        Self::ContactQuadrature,
        Self::ContactWork,
        Self::ContactPathQuadrature,
        Self::MaterialPathQuadrature,
        Self::MidpointWork,
        Self::ViscoelasticRelaxation,
        Self::ViscoelasticHeat,
        Self::SupportMotionDifference,
        Self::SupportLineSearch,
        Self::SupportSearchDirection,
        Self::SupportNonlinear,
        Self::SupportWork,
        Self::SupportPathCrossing,
        Self::FiniteDeformationSupportWork,
        Self::FiniteDeformationEnergy,
        Self::TetrahedralPathCollapse,
        Self::TissueGapPathCrossing,
    ];

    /// The error message returned through the public `&'static str` API.
    pub(crate) const fn message(self) -> &'static str {
        match self {
            Self::ClosedSurfaceGap => "closed surface contact gap",
            Self::ContactNonlinear => "implicit contact nonlinear nonconvergence",
            Self::ContactLineSearch => "implicit contact line search failed",
            Self::ContactQuadrature => "implicit contact quadrature nonconvergence",
            Self::ContactWork => "implicit contact work defect",
            Self::ContactPathQuadrature => "implicit contact path quadrature refinement",
            Self::MaterialPathQuadrature => "implicit material path quadrature refinement",
            Self::MidpointWork => "implicit midpoint work defect",
            Self::ViscoelasticRelaxation => "viscoelastic relaxation energy defect",
            Self::ViscoelasticHeat => "viscoelastic inertial work heat defect",
            Self::SupportMotionDifference => "implicit support motion difference exceeded",
            Self::SupportLineSearch => "implicit support line search failed",
            Self::SupportSearchDirection => "implicit support search direction failed",
            Self::SupportNonlinear => "implicit support nonlinear nonconvergence",
            Self::SupportWork => "implicit support work defect",
            Self::SupportPathCrossing => "implicit support path crossing",
            Self::FiniteDeformationSupportWork => "finite-deformation inertial support work defect",
            Self::FiniteDeformationEnergy => "finite-deformation inertial energy defect",
            Self::TetrahedralPathCollapse => "inertial tetrahedral path collapse",
            Self::TissueGapPathCrossing => "inertial tissue gap path crossing",
        }
    }

    /// The rejection a message names, if it is one of these.
    pub(crate) fn of(message: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|rejection| rejection.message() == message)
    }

    /// A support accuracy interval halves its substep and retries.
    pub(crate) fn refines_support_interval(self) -> bool {
        matches!(
            self,
            Self::SupportMotionDifference
                | Self::SupportLineSearch
                | Self::SupportSearchDirection
                | Self::SupportNonlinear
                | Self::SupportWork
                | Self::SupportPathCrossing
                | Self::FiniteDeformationSupportWork
                | Self::FiniteDeformationEnergy
                | Self::TetrahedralPathCollapse
                | Self::TissueGapPathCrossing
                | Self::ViscoelasticRelaxation
                | Self::ViscoelasticHeat
        )
    }

    /// An adaptive contact interval halves its substep and retries. The committed
    /// state is admitted beforehand, so a closed barrier only in the trial pose
    /// can be retried at a smaller step.
    pub(crate) fn refines_contact_interval(self) -> bool {
        matches!(
            self,
            Self::ClosedSurfaceGap
                | Self::ContactNonlinear
                | Self::ContactLineSearch
                | Self::ContactQuadrature
                | Self::MidpointWork
                | Self::ViscoelasticRelaxation
                | Self::ViscoelasticHeat
        )
    }

    /// The surface quadrature has already refined its panels and asks to rerun.
    pub(crate) fn refines_quadrature_panels(self) -> bool {
        matches!(
            self,
            Self::ContactWork | Self::ContactPathQuadrature | Self::MaterialPathQuadrature
        )
    }

    /// A nonlinear failure that may succeed on finer quadrature panels.
    pub(crate) fn retries_nonlinear_quadrature(self) -> bool {
        matches!(self, Self::ContactNonlinear | Self::ContactLineSearch)
    }
}

#[cfg(test)]
mod tests {
    use super::Rejection;

    #[test]
    fn messages_are_distinct_and_name_their_rejection() {
        for rejection in Rejection::ALL {
            assert_eq!(Rejection::of(rejection.message()), Some(rejection));
        }
        assert_eq!(Rejection::of("unrelated failure"), None);
    }

    #[test]
    fn policies_keep_the_previous_message_lists() {
        let names = |policy: fn(Rejection) -> bool| -> Vec<&str> {
            Rejection::ALL
                .into_iter()
                .filter(|r| policy(*r))
                .map(Rejection::message)
                .collect()
        };
        assert_eq!(names(Rejection::refines_contact_interval).len(), 7);
        assert_eq!(names(Rejection::refines_support_interval).len(), 12);
        assert_eq!(
            names(Rejection::refines_quadrature_panels),
            [
                "implicit contact work defect",
                "implicit contact path quadrature refinement",
                "implicit material path quadrature refinement"
            ]
        );
        assert_eq!(
            names(Rejection::retries_nonlinear_quadrature),
            [
                "implicit contact nonlinear nonconvergence",
                "implicit contact line search failed"
            ]
        );
    }
}
