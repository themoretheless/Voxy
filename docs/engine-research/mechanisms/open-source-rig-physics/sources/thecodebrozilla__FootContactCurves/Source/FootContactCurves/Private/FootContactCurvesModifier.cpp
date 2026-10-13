// Copyright (c) 2026 Shane Beal. Licensed under the MIT License.

#include "FootContactCurvesModifier.h"

#include "Animation/AnimSequence.h"
#include "Animation/AnimCurveTypes.h"
#include "Animation/AnimData/CurveIdentifier.h"
#include "Animation/AnimData/IAnimationDataController.h"
#include "AnimPose.h"
#include "Curves/RichCurve.h"
#include "Logging/LogMacros.h"

DEFINE_LOG_CATEGORY_STATIC(LogFootContactCurves, Log, All);

#define LOCTEXT_NAMESPACE "FootContactCurvesModifier"

UFootContactCurvesModifier::UFootContactCurvesModifier()
{
}

void UFootContactCurvesModifier::OnApply_Implementation(UAnimSequence* AnimationSequence)
{
	Super::OnApply_Implementation(AnimationSequence);

	if (!AnimationSequence)
	{
		UE_LOG(LogFootContactCurves, Warning, TEXT("OnApply: null AnimationSequence."));
		return;
	}

	if (!bBakeLeftFoot && !bBakeRightFoot)
	{
		UE_LOG(LogFootContactCurves, Warning,
			TEXT("OnApply on '%s': both feet are disabled; nothing to bake."),
			*AnimationSequence->GetName());
		return;
	}

	// Disable root-motion lock during sampling so foot bones evaluate consistently.
	TGuardValue<bool> ForceRootLockGuard(AnimationSequence->bForceRootLock, false);

	if (bBakeLeftFoot)
	{
		BakeContactCurve(AnimationSequence, FootBoneName_L, CurveName_L);
	}

	if (bBakeRightFoot)
	{
		BakeContactCurve(AnimationSequence, FootBoneName_R, CurveName_R);
	}
}

void UFootContactCurvesModifier::OnRevert_Implementation(UAnimSequence* AnimationSequence)
{
	Super::OnRevert_Implementation(AnimationSequence);

	if (!AnimationSequence)
	{
		return;
	}

	IAnimationDataController& Controller = AnimationSequence->GetController();
	Controller.OpenBracket(LOCTEXT("RevertFootContactCurves", "Revert Foot Contact Curves"));

	if (bBakeLeftFoot)
	{
		Controller.RemoveCurve(FAnimationCurveIdentifier(CurveName_L, ERawCurveTrackTypes::RCT_Float));
	}
	if (bBakeRightFoot)
	{
		Controller.RemoveCurve(FAnimationCurveIdentifier(CurveName_R, ERawCurveTrackTypes::RCT_Float));
	}

	Controller.CloseBracket();
}

void UFootContactCurvesModifier::BakeContactCurve(UAnimSequence* AnimationSequence, FName FootBone, FName CurveName) const
{
	if (FootBone.IsNone() || CurveName.IsNone())
	{
		UE_LOG(LogFootContactCurves, Warning,
			TEXT("BakeContactCurve: foot bone or curve name not set on '%s'; skipping."),
			*GetNameSafe(AnimationSequence));
		return;
	}

	const float SequenceLength = AnimationSequence->GetPlayLength();
	if (SequenceLength <= 0.f || SampleRate <= 0)
	{
		UE_LOG(LogFootContactCurves, Warning,
			TEXT("BakeContactCurve: invalid sequence length (%.3f) or sample rate (%d) on '%s'."),
			SequenceLength, SampleRate, *GetNameSafe(AnimationSequence));
		return;
	}

	const float SampleStep = 1.0f / FMath::Max(1.0f, static_cast<float>(SampleRate));
	const int32 SampleCount = FMath::Max(2, FMath::TruncToInt(SequenceLength / SampleStep) + 1);

	FAnimPoseEvaluationOptions EvalOptions;
	EvalOptions.EvaluationType = EAnimDataEvalType::Raw;
	EvalOptions.bShouldRetarget = true;
	EvalOptions.bExtractRootMotion = false;
	EvalOptions.bIncorporateRootMotionIntoPose = false;
	EvalOptions.bRetrieveAdditiveAsFullPose = true;
	EvalOptions.bEvaluateCurves = false;

	// Pass 1: sample foot Z per frame in component (world) space; find the ground level (min Z over the clip).
	TArray<float> FootZ;
	FootZ.SetNumZeroed(SampleCount);

	float GroundLevel = TNumericLimits<float>::Max();

	for (int32 Sample = 0; Sample < SampleCount; ++Sample)
	{
		const float Time = FMath::Clamp(static_cast<float>(Sample) * SampleStep, 0.0f, SequenceLength);

		FAnimPose Pose;
		UAnimPoseExtensions::GetAnimPoseAtTime(AnimationSequence, Time, EvalOptions, Pose);

		if (!Pose.IsValid())
		{
			UE_LOG(LogFootContactCurves, Warning,
				TEXT("BakeContactCurve: invalid pose at time %.3f on '%s'; aborting."),
				Time, *GetNameSafe(AnimationSequence));
			return;
		}

		const FTransform FootTransform = UAnimPoseExtensions::GetBonePose(Pose, FootBone, EAnimPoseSpaces::World);
		FootZ[Sample] = FootTransform.GetLocation().Z;
		GroundLevel = FMath::Min(GroundLevel, FootZ[Sample]);
	}

	// Pass 2: classify each frame as planted (1) or in-air (0).
	TArray<bool> Contact;
	Contact.SetNumZeroed(SampleCount);

	for (int32 Sample = 0; Sample < SampleCount; ++Sample)
	{
		const float HeightAboveGround = FootZ[Sample] - GroundLevel;

		float SpeedZ;
		if (Sample == 0 && SampleCount > 1)
		{
			SpeedZ = FMath::Abs(FootZ[1] - FootZ[0]) / SampleStep;
		}
		else if (Sample > 0)
		{
			SpeedZ = FMath::Abs(FootZ[Sample] - FootZ[Sample - 1]) / SampleStep;
		}
		else
		{
			SpeedZ = 0.f;
		}

		Contact[Sample] = (HeightAboveGround < GroundHeightThreshold) && (SpeedZ < VerticalSpeedThreshold);
	}

	// Build curve keys.
	TArray<FRichCurveKey> Keys;
	Keys.Reserve(SampleCount);

	for (int32 Sample = 0; Sample < SampleCount; ++Sample)
	{
		const float Time = FMath::Clamp(static_cast<float>(Sample) * SampleStep, 0.0f, SequenceLength);
		const float Value = Contact[Sample] ? 1.0f : 0.0f;

		FRichCurveKey Key(Time, Value);
		Key.InterpMode = InterpolationMode;
		Keys.Add(Key);
	}

	// Write the curve through the data controller (transactional, undoable).
	IAnimationDataController& Controller = AnimationSequence->GetController();
	Controller.OpenBracket(LOCTEXT("BakeFootContactCurve", "Bake Foot Contact Curve"));

	const FAnimationCurveIdentifier CurveId(CurveName, ERawCurveTrackTypes::RCT_Float);

	if (bOverwriteExistingCurves)
	{
		Controller.RemoveCurve(CurveId);
	}

	Controller.AddCurve(CurveId, AACF_DefaultCurve);
	Controller.SetCurveKeys(CurveId, Keys);

	Controller.CloseBracket();

	int32 PlantedFrames = 0;
	for (bool b : Contact)
	{
		if (b) { ++PlantedFrames; }
	}

	UE_LOG(LogFootContactCurves, Display,
		TEXT("Baked '%s' on '%s': %d samples, %d planted (%.1f%%), ground=%.2fcm, height_thr=%.2fcm, vspeed_thr=%.2fcm/s."),
		*CurveName.ToString(),
		*AnimationSequence->GetName(),
		SampleCount,
		PlantedFrames,
		100.f * static_cast<float>(PlantedFrames) / FMath::Max(1, SampleCount),
		GroundLevel,
		GroundHeightThreshold,
		VerticalSpeedThreshold);
}

#undef LOCTEXT_NAMESPACE
