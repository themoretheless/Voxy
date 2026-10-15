# Facial rendering source scan: MiguelCPereira/SuperMario64Remake

Repository: https://github.com/MiguelCPereira/SuperMario64Remake

Pinned source revision: 7d2df7f65fa6814d1afcaabdb579de1ff5e98dd1

Discovery topic: skin shader

Description: A DirectX remake of Super Mario 64's first level - showcasing Hardware Skinning, Shadow Mapping, simple particle systems and other HLSL shader work. 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"skin": ["Build/Resources/Effects/PosNormTex3D_Skinned.fx", "Build/Resources/Effects/Shadow/PosNormTex3D_Shadow_Skinned.fx", "Source/OverlordProject/Materials/DiffuseMaterial_Skinned.cpp", "Source/OverlordProject/Materials/DiffuseMaterial_Skinned.h", "Source/OverlordProject/Materials/Shadow/DiffuseMaterial_Shadow_Skinned.cpp", "Source/OverlordProject/Materials/Shadow/DiffuseMaterial_Shadow_Skinned.h", "Source/OverlordProject/Resources/Effects/PosNormTex3D_Skinned.fx", "Source/OverlordProject/Resources/Effects/Shadow/PosNormTex3D_Shadow_Skinned.fx", "Source/OverlordProject/Scenes/Week 6/SoftwareSkinningScene_1.cpp", "Source/OverlordProject/Scenes/Week 6/SoftwareSkinningScene_1.h", "Source/OverlordProject/Scenes/Week 6/SoftwareSkinningScene_2.cpp", "Source/OverlordProject/Scenes/Week 6/SoftwareSkinningScene_2.h", "Source/OverlordProject/Scenes/Week 6/SoftwareSkinningScene_3.cpp", "Source/OverlordProject/Scenes/Week 6/SoftwareSkinningScene_3.h", "Source/OverlordProject/Scenes/Week 7/HardwareSkinningScene.cpp", "Source/OverlordProject/Scenes/Week 7/HardwareSkinningScene.h"], "shader": ["Source/OverlordEngine/PhysX/OverlordSimulationFilterShader.h"], "rig": ["Source/OverlordEngine/Components/RigidBodyComponent.cpp", "Source/OverlordEngine/Components/RigidBodyComponent.h"]}

## Source file: Source/OverlordEngine/Components/RigidBodyComponent.h

{
  "path": "Source/OverlordEngine/Components/RigidBodyComponent.h",
  "blob_sha": "98ef60f7be90fc7f85047401344dec1084f5f9b6",
  "sha256": "7d038fb606d8ecc65228ad80ceeb1ac322f1c61d56c9a49e7ec8dfd291be1189",
  "lines": 135,
  "bytes": 4507,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 3,
    "rig": 27
  },
  "url": "https://github.com/MiguelCPereira/SuperMario64Remake/blob/7d2df7f65fa6814d1afcaabdb579de1ff5e98dd1/Source/OverlordEngine/Components/RigidBodyComponent.h",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "bool IsTrigger() const { return m_IsTrigger; }"
    },
    {
      "line": 18,
      "text": "void SetTag(const std::wstring& tag) { m_Tag = tag; }"
    },
    {
      "line": 19,
      "text": "void SetTrigger(bool isTrigger)"
    },
    {
      "line": 56,
      "text": "bool IsStatic()  const { return m_IsStatic; }"
    },
    {
      "line": 57,
      "text": "bool IsDynamic()  const { return !m_IsStatic; }"
    },
    {
      "line": 58,
      "text": "bool IsKinematic() const { return m_IsKinematic; }"
    },
    {
      "line": 59,
      "text": "void SetKinematic(bool isKinematic);"
    },
    {
      "line": 60,
      "text": "void PutToSleep() const;"
    },
    {
      "line": 61,
      "text": "void WakeUp() const;"
    },
    {
      "line": 64,
      "text": "void AddForce(const XMFLOAT3& force, PxForceMode::Enum mode = PxForceMode::eFORCE, bool autowake = true) const;"
    },
    {
      "line": 65,
      "text": "void AddTorque(const XMFLOAT3& torque, PxForceMode::Enum mode = PxForceMode::eFORCE, bool autowake = true) const;"
    },
    {
      "line": 66,
      "text": "void ClearForce(PxForceMode::Enum mode = PxForceMode::eFORCE) const;"
    },
    {
      "line": 67,
      "text": "void ClearTorque(PxForceMode::Enum mode = PxForceMode::eFORCE) const;"
    },
    {
      "line": 69,
      "text": "void SetConstraint(RigidBodyConstraint flag, bool enable);"
    },
    {
      "line": 70,
      "text": "void SetCollisionGroup(CollisionGroup group);"
    },
    {
      "line": 71,
      "text": "void SetCollisionIgnoreGroups(CollisionGroup ignoreGroups);"
    },
    {
      "line": 72,
      "text": "void SetDensity(float density) const;"
    },
    {
      "line": 75,
      "text": "void Translate(const XMFLOAT3& position) const;"
    },
    {
      "line": 76,
      "text": "void Rotate(const XMFLOAT4& rotation) const;"
    },
    {
      "line": 83,
      "text": "void RemoveCollider(const ColliderInfo& colliderInfo);"
    },
    {
      "line": 84,
      "text": "void RemoveColliders();"
    },
    {
      "line": 90,
      "text": "void Initialize(const SceneContext& /*sceneContext*/) override;"
    },
    {
      "line": 91,
      "text": "void OnSceneAttach(GameScene* pScene) override;"
    },
    {
      "line": 92,
      "text": "void OnSceneDetach(GameScene* pScene) override;"
    },
    {
      "line": 93,
      "text": "void OnOwnerAttach(GameObject*) override;"
    },
    {
      "line": 94,
      "text": "void OnOwnerDetach(GameObject*) override;"
    },
    {
      "line": 118,
      "text": "void CreateActor();"
    }
  ]
}

```text

#pragma once

#pragma region ColliderInfo
class ColliderInfo
{
public:
	ColliderInfo(UINT Id, RigidBodyComponent* pOwner, PxShape* pShape) :
		m_ColliderId{Id},
		m_pOwner{pOwner},
		m_pShape(pShape){}

	PxShape* GetShape() const { return m_pShape; }
	bool IsTrigger() const { return m_IsTrigger; }
	const std::wstring& GetTag() const { return m_Tag; }
	RigidBodyComponent* GetOwner() const { return m_pOwner; }
	UINT GetColliderId() const { return m_ColliderId; }

	void SetTag(const std::wstring& tag) { m_Tag = tag; }
	void SetTrigger(bool isTrigger)
	{
		if(m_pShape && m_IsTrigger != isTrigger)
		{
			m_IsTrigger = isTrigger;
			m_pShape->setFlag(PxShapeFlag::eSIMULATION_SHAPE, !m_IsTrigger);
			m_pShape->setFlag(PxShapeFlag::eTRIGGER_SHAPE, m_IsTrigger);
		}
	}

	static ColliderInfo INVALID;

private:
	RigidBodyComponent* m_pOwner{};
	PxShape* m_pShape{};
	bool m_IsTrigger{};
	std::wstring m_Tag{};
	UINT m_ColliderId{ UINT_MAX };
};
inline bool operator==(const ColliderInfo& lhs, const ColliderInfo& rhs)
{
	return lhs.GetOwner() == rhs.GetOwner() && lhs.GetShape() == rhs.GetShape();
}
#pragma endregion

class RigidBodyComponent final : public BaseComponent
{
public:
	RigidBodyComponent(bool isStatic = false);
	~RigidBodyComponent() override;

	RigidBodyComponent(const RigidBodyComponent& other) = delete;
	RigidBodyComponent(RigidBodyComponent&& other) noexcept = delete;
	RigidBodyComponent& operator=(const RigidBodyComponent& other) = delete;
	RigidBodyComponent& operator=(RigidBodyComponent&& other) noexcept = delete;


	bool IsStatic()  const { return m_IsStatic; }
	bool IsDynamic()  const { return !m_IsStatic; }
	bool IsKinematic() const { return m_IsKinematic; }
	void SetKinematic(bool isKinematic);
	void PutToSleep() const;
	void WakeUp() const;

	PxRigidActor* GetPxRigidActor() const { return m_pActor; };
	void AddForce(const XMFLOAT3& force, PxForceMode::Enum mode = PxForceMode::eFORCE, bool autowake = true) const;
	void AddTorque(const XMFLOAT3& torque, PxForceMode::Enum mode = PxForceMode::eFORCE, bool autowake = true) const;
	void ClearForce(PxForceMode::Enum mode = PxForceMode::eFORCE) const;
	void ClearTorque(PxForceMode::Enum mode = PxForceMode::eFORCE) const;

	void SetConstraint(RigidBodyConstraint flag, bool enable);
	void SetCollisionGroup(CollisionGroup group);
	void SetCollisionIgnoreGroups(CollisionGroup ignoreGroups);
	void SetDensity(float density) const;

	//Internal Use (use Transform Component for Transformations)
	void Translate(const XMFLOAT3& position) const;
	void Rotate(const XMFLOAT4& rotation) const;
	XMFLOAT3 GetPosition() const;
	XMFLOAT4 GetRotation() const;

	//Shapes (Colliders)
	template<typename T>
	UINT AddCollider(const T& geometry, const PxMaterial& material, bool isTrigger=false, const PxTransform& localPose = PxTransform(PxIdentity));
	void RemoveCollider(const ColliderInfo& colliderInfo);
	void RemoveColliders();
	const std::vector<ColliderInfo>& GetColliders() const { return m_Colliders; }
	ColliderInfo& GetCollider(UINT colliderId);

protected:

	void Initialize(const SceneContext& /*sceneContext*/) override;
	void OnSceneAttach(GameScene* pScene) override;
	void OnSceneDetach(GameScene* pScene) override;
	void OnOwnerAttach(GameObject*) override;
	void OnOwnerDetach(GameObject*) override;

private:
	//Temporary Storage data
	struct ColliderCreationInfo
	{
		UINT colliderId{};
		const std::shared_ptr<PxGeometry> pGeometry{};
		const PxMaterial* pMaterial{};
		PxTransform localPose{};
		bool isTrigger{};
	};
	std::vector<ColliderCreationInfo> m_ColliderCreationInfos{};

	PxRigidActor* m_pActor{};
	std::vector<ColliderInfo> m_Colliders{};

	bool m_IsStatic{};
	bool m_IsKinematic{};

	PxFilterData m_CollisionGroups{};

	RigidBodyConstraint m_InitialConstraints{};

	void CreateActor();
	UINT _AddCollider(const PxGeometry& geometry, const PxMaterial& material, bool isTrigger = false, const PxTransform& localPose = PxTransform(PxIdentity), UINT colliderId = UINT_MAX);
};

template <typename T>
UINT RigidBodyComponent::AddCollider(const T& geometry, const PxMaterial& material, bool isTrigger, const PxTransform& localPose)
{
	if (!m_pActor)
	{
		const UINT colliderId = UINT(m_ColliderCreationInfos.size());
		const std::shared_ptr<PxGeometry> pGeom(new T{ geometry }); //Prevent PxGeometry Object Slicing...

		m_ColliderCreationInfos.push_back({ colliderId, pGeom, &material, localPose, isTrigger});
		return colliderId;
	}

	return _AddCollider(geometry, material, isTrigger, localPose);
}


```

## Source file: Source/OverlordEngine/Components/RigidBodyComponent.cpp

{
  "path": "Source/OverlordEngine/Components/RigidBodyComponent.cpp",
  "blob_sha": "0b272308e45f16895743fea264cc8c1f1b703473",
  "sha256": "c93f7a40dc3e10a23ba08eb45f6285af6e698739f544d9babe45f3e11d5b1edf",
  "lines": 337,
  "bytes": 9689,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 14,
    "rig": 70
  },
  "url": "https://github.com/MiguelCPereira/SuperMario64Remake/blob/7d2df7f65fa6814d1afcaabdb579de1ff5e98dd1/Source/OverlordEngine/Components/RigidBodyComponent.cpp",
  "function_signature_candidates": []
}

```text

#include "stdafx.h"
#include "RigidBodyComponent.h"

RigidBodyComponent::RigidBodyComponent(bool isStatic) :
	m_IsStatic(isStatic),
	m_CollisionGroups(PxFilterData(static_cast<UINT32>(CollisionGroup::Group0), 0, 0, 1))
{
}

RigidBodyComponent::~RigidBodyComponent()
{
	if (m_pActor != nullptr)
	{
		//Only release actor if not part of a pxScene
		if (!m_pActor->getScene()) m_pActor->release();
	}
}

ColliderInfo& RigidBodyComponent::GetCollider(UINT colliderId)
{
	ASSERT_IF(colliderId >= m_Colliders.size(), L"Collider with Id={} cannot be retrieved. (Make sure the RigidBody is initialized before querying a collider.", colliderId);

	return m_Colliders[colliderId];
}

void RigidBodyComponent::Initialize(const SceneContext&)
{
	if(!m_pActor)
		CreateActor();

	//Create delayed colliders
	for(const auto& cci : m_ColliderCreationInfos)
	{
		_AddCollider(*cci.pGeometry, *cci.pMaterial, cci.isTrigger, cci.localPose);
	}
	m_ColliderCreationInfos.clear();
}

void RigidBodyComponent::OnSceneAttach(GameScene* pScene)
{
	if(m_pActor && !m_pActor->getScene())
	{
		pScene->GetPhysxProxy()->GetPhysxScene()->addActor(*m_pActor);
	}
}

void RigidBodyComponent::OnSceneDetach(GameScene* /*pScene*/)
{
	if(m_pActor)
	{
		if (const auto pxScene = m_pActor->getScene())
		{
			pxScene->removeActor(*m_pActor); //Remove actor from pxScene if component is detached from scenegraph
		}
	}
}

void RigidBodyComponent::OnOwnerAttach(GameObject* pOwner)
{
	pOwner->GetTransform()->SetRigidBodyComponent(this);
}

void RigidBodyComponent::OnOwnerDetach(GameObject* pOwner)
{
	pOwner->GetTransform()->SetRigidBodyComponent(nullptr);
}

#pragma region Collider Logic
UINT RigidBodyComponent::_AddCollider(const PxGeometry& geometry, const PxMaterial& material, bool isTrigger, const PxTransform& localPose, UINT colliderId)
{
	ASSERT_NULL_(m_pActor); //This version of AddCollider can only be called if m_pActor exists

	PxShape* pShape = PxRigidActorExt::createExclusiveShape(*m_pActor, geometry, material);
	pShape->setLocalPose(localPose);
	pShape->setSimulationFilterData(m_CollisionGroups);
	pShape->setQueryFilterData(m_CollisionGroups);

	if(colliderId == UINT_MAX)
	{
		colliderId = UINT(m_ColliderCreationInfos.size() + m_Colliders.size());
	}

	m_Colliders.emplace_back(colliderId, this, pShape);
	m_Colliders.back().SetTrigger(isTrigger);

	return colliderId;
}

void RigidBodyComponent::RemoveCollider(const ColliderInfo& colliderInfo)
{
	if(colliderInfo.GetOwner() != this)
	{
		Logger::LogWarning(L"[RigidBodyComponent] Failed to remove the given collider. (Incorrect owner)");
		return;
	}

	auto it = std::find(m_Colliders.begin(), m_Colliders.end(), colliderInfo);
	if(it == m_Colliders.end())
	{
		Logger::LogWarning(L"[RigidBodyComponent] Failed to remove the given collider. (Collider not found on owner)");
		return;
	}

	PxShape* pShape = colliderInfo.GetShape();
	m_pActor->detachShape(*pShape);
	pShape->release();
	m_Colliders.erase(it);
}

void RigidBodyComponent::RemoveColliders()
{
	for(auto& colliderInfo:m_Colliders)
	{
		PxShape* pShape = colliderInfo.GetShape();
		m_pActor->detachShape(*pShape);
		pShape->release();
	}

	m_Colliders.clear();
}
#pragma endregion

void RigidBodyComponent::SetCollisionIgnoreGroups(CollisionGroup ignoreGroups)
{
	m_CollisionGroups.word1 = static_cast<UINT32>(ignoreGroups);
	for(auto& colliderInfo : m_Colliders)
	{
		colliderInfo.GetShape()->setSimulationFilterData(m_CollisionGroups);
	}
}

void RigidBodyComponent::SetCollisionGroup(CollisionGroup group)
{
	m_CollisionGroups.word0 = static_cast<UINT32>(group);
	for (auto& colliderInfo : m_Colliders)
	{
		colliderInfo.GetShape()->setSimulationFilterData(m_CollisionGroups);
		colliderInfo.GetShape()->setQueryFilterData(m_CollisionGroups);
	}
}

void RigidBodyComponent::SetKinematic(bool isKinematic)
{
	if (m_IsStatic && isKinematic) {
		Logger::LogWarning(L"A static RigidBody can never be kinematic");
		return;
	}

	m_IsKinematic = isKinematic;

	if (m_pActor != nullptr && !m_IsStatic)
	{
		m_pActor->is<PxRigidDynamic>()->setRigidBodyFlag(PxRigidBodyFlag::eKINEMATIC, m_IsKinematic);
	}
}

void RigidBodyComponent::CreateActor()
{
	ASSERT_IF(m_pActor != nullptr, L"CreateActor cannot be called multiple times")

	const auto pPhysX = PhysXManager::Get()->GetPhysics();
	const auto pPhysxScene = GetGameObject()->GetScene()->GetPhysxProxy()->GetPhysxScene();
	const auto pTransform = GetTransform();

	if (m_IsStatic)
		m_pActor = pPhysX->createRigidStatic(PxTransform(PhysxHelper::ToPxVec3(pTransform->GetPosition()), PhysxHelper::ToPxQuat(pTransform->GetRotation())));
	else
	{
		m_pActor = pPhysX->createRigidDynamic(PxTransform(PhysxHelper::ToPxVec3(pTransform->GetPosition()), PhysxHelper::ToPxQuat(pTransform->GetRotation())));
		m_pActor->is<PxRigidDynamic>()->setRigidBodyFlag(PxRigidBodyFlag::eKINEMATIC, m_IsKinematic);

		if (m_InitialConstraints != RigidBodyConstraint::None)
		{
			SetConstraint(RigidBodyConstraint::All, true);
			SetConstraint(m_InitialConstraints, false);
		}
	}

	pPhysxScene->addActor(*m_pActor);
	m_pActor->userData = this;
}

void RigidBodyComponent::SetDensity(float density) const
{
	if (m_pActor && m_pActor->is<PxRigidDynamic>())
	{
		PxRigidBodyExt::updateMassAndInertia(*m_pActor->is<PxRigidDynamic>(), density);
	}
}

void RigidBodyComponent::Translate(const XMFLOAT3& position) const
{
	ASSERT_NULL_(m_pActor)
	const PxTransform localPose = PxTransform(PhysxHelper::ToPxVec3(position), PhysxHelper::ToPxQuat(GetRotation()));

	if (!m_IsKinematic)
	{
		m_pActor->setGlobalPose(localPose);

		if (!m_IsStatic)
			m_pActor->is<PxRigidDynamic>()->setLinearVelocity(PxVec3{ PxZero }); //Non-Kinematic Only
	}
	else
		m_pActor->is<PxRigidDynamic>()->setKinematicTarget(localPose);
}

void RigidBodyComponent::Rotate(const XMFLOAT4& rotation) const
{
	ASSERT_NULL_(m_pActor)
	const PxTransform localPose = PxTransform(PhysxHelper::ToPxVec3(GetPosition()), PhysxHelper::ToPxQuat(rotation));

	if (!m_IsKinematic) {
		m_pActor->setGlobalPose(localPose);

		if(!m_IsStatic)
			m_pActor->is<PxRigidDynamic>()->setAngularVelocity(PxVec3{ PxZero }); //Non-Kinematic Only!
	}
	else
		m_pActor->is<PxRigidDynamic>()->setKinematicTarget(localPose);

}

XMFLOAT3 RigidBodyComponent::GetPosition() const
{
	ASSERT_NULL_(m_pActor);

	PxTransform pose;
	if (!m_IsKinematic || !m_pActor->is<PxRigidDynamic>()->getKinematicTarget(pose))
		pose = m_pActor->getGlobalPose();

	return { pose.p.x, pose.p.y, pose.p.z };
}

XMFLOAT4 RigidBodyComponent::GetRotation() const
{
	ASSERT_NULL_(m_pActor);
	PxTransform pose;
	if (!m_IsKinematic || !reinterpret_cast<PxRigidDynamic*>(m_pActor)->getKinematicTarget(pose))
		pose = m_pActor->getGlobalPose();

	return { pose.q.x, pose.q.y, pose.q.z, pose.q.w };
}

void RigidBodyComponent::PutToSleep() const
{
	if (m_pActor != nullptr && !m_IsStatic)
		m_pActor->is<PxRigidDynamic>()->putToSleep();
	else
		Logger::LogWarning(L"Cannot put a static actor to sleep");
}

void RigidBodyComponent::WakeUp() const
{
	if (m_pActor != nullptr && !m_IsStatic)
		m_pActor->is<PxRigidDynamic>()->wakeUp();
	else
		Logger::LogWarning(L"Cannot wake-up a static actor");
}

#pragma warning(push)
#pragma warning(disable: 26812)
void RigidBodyComponent::AddForce(const XMFLOAT3& force, PxForceMode::Enum mode, bool autowake) const
{
	if (m_pActor != nullptr && !m_IsStatic && !m_IsKinematic)
		m_pActor->is<PxRigidDynamic>()->addForce(PhysxHelper::ToPxVec3(force), mode, autowake);
	else
		Logger::LogWarning(L"Cannot apply a force on a static or kinematic actor");
}
#pragma warning(pop)

void RigidBodyComponent::AddTorque(const XMFLOAT3& torque, PxForceMode::Enum mode, bool autowake) const
{
	if (m_pActor != nullptr && !m_IsStatic && !m_IsKinematic)
		m_pActor->is<PxRigidDynamic>()->addTorque(PhysxHelper::ToPxVec3(torque), mode, autowake);
	else
		Logger::LogWarning(L"Cannot apply torque on a static or kinematic actor");
}

void RigidBodyComponent::ClearForce(PxForceMode::Enum mode) const
{
	if (m_pActor != nullptr && !m_IsStatic && !m_IsKinematic)
		m_pActor->is<PxRigidDynamic>()->clearForce(mode);
	else
		Logger::LogWarning(L"Cannot clear force on a static or kinematic actor");
}

void RigidBodyComponent::ClearTorque(PxForceMode::Enum mode) const
{
	if (m_pActor != nullptr && !m_IsStatic && !m_IsKinematic)
		m_pActor->is<PxRigidDynamic>()->clearTorque(mode);
	else
		Logger::LogWarning(L"Cannot clear torque on a static or kinematic actor");
}

void RigidBodyComponent::SetConstraint(RigidBodyConstraint flag, bool enable)
{
	if (m_IsStatic || m_IsKinematic)
	{
		Logger::LogWarning(L"Can't constrain a static or kinematic actor!");
		return;
	}

	if(m_pActor)
	{
		//TODO: Refactor!!
		const auto pRigidBody = m_pActor->is<PxRigidDynamic>();
		if (isSet(flag, RigidBodyConstraint::RotX))
		{
			pRigidBody->setRigidDynamicLockFlag(PxRigidDynamicLockFlag::eLOCK_ANGULAR_X, !enable);
		}
		if (isSet(flag, RigidBodyConstraint::RotY))
		{
			pRigidBody->setRigidDynamicLockFlag(PxRigidDynamicLockFlag::eLOCK_ANGULAR_Y, !enable);
		}
		if (isSet(flag, RigidBodyConstraint::RotZ))
		{
			pRigidBody->setRigidDynamicLockFlag(PxRigidDynamicLockFlag::eLOCK_ANGULAR_Z, !enable);
		}
		if (isSet(flag, RigidBodyConstraint::TransX))
		{
			pRigidBody->setRigidDynamicLockFlag(PxRigidDynamicLockFlag::eLOCK_LINEAR_X, !enable);
		}
		if (isSet(flag, RigidBodyConstraint::TransY))
		{
			pRigidBody->setRigidDynamicLockFlag(PxRigidDynamicLockFlag::eLOCK_LINEAR_Y, !enable);
		}
		if (isSet(flag, RigidBodyConstraint::TransZ))
		{
			pRigidBody->setRigidDynamicLockFlag(PxRigidDynamicLockFlag::eLOCK_LINEAR_Z, !enable);
		}
	}
	else
	{
		if (!enable) m_InitialConstraints |= flag;
		else m_InitialConstraints &= ~flag;
	}
}


```

## Source file: Source/OverlordProject/Materials/DiffuseMaterial_Skinned.h

{
  "path": "Source/OverlordProject/Materials/DiffuseMaterial_Skinned.h",
  "blob_sha": "78c1d6f5d9d79fde01cac14b25c8ab57671651bc",
  "sha256": "10f0e5678a0913f8ad4a783bec2a9b76e7d961f86c10792de6fda758e8506622",
  "lines": 22,
  "bytes": 764,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 7
  },
  "url": "https://github.com/MiguelCPereira/SuperMario64Remake/blob/7d2df7f65fa6814d1afcaabdb579de1ff5e98dd1/Source/OverlordProject/Materials/DiffuseMaterial_Skinned.h",
  "function_signature_candidates": [
    {
      "line": 13,
      "text": "void SetDiffuseTexture(const std::wstring& assetFile);"
    },
    {
      "line": 16,
      "text": "void InitializeEffectVariables() override;"
    },
    {
      "line": 17,
      "text": "void OnUpdateModelVariables(const SceneContext& /*sceneContext*/, const ModelComponent* /*pModel*/) const;"
    }
  ]
}

```text

#pragma once
class DiffuseMaterial_Skinned: public Material<DiffuseMaterial_Skinned>
{
public:
	DiffuseMaterial_Skinned();
	~DiffuseMaterial_Skinned() override = default;

	DiffuseMaterial_Skinned(const DiffuseMaterial_Skinned& other) = delete;
	DiffuseMaterial_Skinned(DiffuseMaterial_Skinned&& other) noexcept = delete;
	DiffuseMaterial_Skinned& operator=(const DiffuseMaterial_Skinned& other) = delete;
	DiffuseMaterial_Skinned& operator=(DiffuseMaterial_Skinned&& other) noexcept = delete;

	void SetDiffuseTexture(const std::wstring& assetFile);

protected:
	void InitializeEffectVariables() override;
	void OnUpdateModelVariables(const SceneContext& /*sceneContext*/, const ModelComponent* /*pModel*/) const;

private:
	TextureData* m_pDiffuseTexture{};
};



```

## Source file: Source/OverlordEngine/PhysX/OverlordSimulationFilterShader.h

{
  "path": "Source/OverlordEngine/PhysX/OverlordSimulationFilterShader.h",
  "blob_sha": "bff958d23fa9ce2bb98c066be1f7b8deaec9116f",
  "sha256": "9c0f505b030ca7220ce9570bead096fb3fdc057c657464e3f7b4cfcf2964387a",
  "lines": 60,
  "bytes": 2077,
  "selection_score": 15,
  "keyword_line_counts": {
    "shader": 4,
    "rig": 7
  },
  "url": "https://github.com/MiguelCPereira/SuperMario64Remake/blob/7d2df7f65fa6814d1afcaabdb579de1ff5e98dd1/Source/OverlordEngine/PhysX/OverlordSimulationFilterShader.h",
  "function_signature_candidates": []
}

```text

#pragma once
#define FILTERSHADERTYPE_CUSTOM

inline PxFilterFlags OverlordSimulationFilterShader(
	PxFilterObjectAttributes attribute0, PxFilterData filterData0,
	PxFilterObjectAttributes attribute1, PxFilterData filterData1,
	PxPairFlags& pairFlags, const void* pConstantBlock, PxU32 blockSize)
{
	//If word3 is not set use default filter_shader (means one of the actors is not a Overlord RigidBody)
	if(filterData0.word3 == 0 || filterData1.word3 == 0)
	{
		return PxDefaultSimulationFilterShader(attribute0, filterData0, attribute1, filterData1, pairFlags, pConstantBlock, blockSize);
	}

	if ((filterData0.word0&filterData1.word1) == filterData0.word0 || (filterData1.word0&filterData0.word1) == filterData1.word0)
	{
			return PxFilterFlag::eSUPPRESS;
	}

	if ((attribute0 & PxFilterObjectFlag::eTRIGGER) != 0 || (attribute1 & PxFilterObjectFlag::eTRIGGER) != 0)
	{
		//pairFlags |= PxPairFlag::eNOTIFY_TOUCH_LOST;
		pairFlags |= PxPairFlag::eTRIGGER_DEFAULT;
		return PxFilterFlag::eDEFAULT;
	}

	if((filterData0.word0 & filterData1.word0) != 0)
	{
		pairFlags |= PxPairFlag::eNOTIFY_TOUCH_FOUND;
		//pairFlags |= PxPairFlag::eNOTIFY_TOUCH_LOST;
	}

	pairFlags |= PxPairFlag::eCONTACT_DEFAULT;

	return PxFilterFlag::eDEFAULT;
}

//PxFilterFlags DefaultSimulationFilter(
//	PxFilterObjectAttributes attribute0, PxFilterData filterData0,
//	PxFilterObjectAttributes attribute1, PxFilterData filterData1,
//	PxPairFlags& pairFlags, const void* , PxU32 )
//{
//	// let triggers through
//	if (PxFilterObjectIsTrigger(attribute0) || PxFilterObjectIsTrigger(attribute1))
//	{
//		pairFlags = PxPairFlag::eTRIGGER_DEFAULT;
//		return PxFilterFlag::eDEFAULT;
//	}
//	// generate contacts for all that were not filtered above
//	pairFlags = PxPairFlag::eCONTACT_DEFAULT;
//
//	// trigger the contact callback for pairs (A,B) where
//	// the filtermask of A contains the ID of B and vice versa.
//	if ((filterData0.word0 & filterData1.word1) && (filterData1.word0 & filterData0.word1))
//		pairFlags |= PxPairFlag::eNOTIFY_TOUCH_FOUND;
//
//	return PxFilterFlag::eDEFAULT;
//}




```
