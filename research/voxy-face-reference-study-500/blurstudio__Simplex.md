# Facial rendering source scan: blurstudio/Simplex

Repository: https://github.com/blurstudio/Simplex

Pinned source revision: 377477f96ea435641b2d2ed2eb6c26cb7bfc96c4

Discovery topic: previous-100

Description: 

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"blendshape": ["src/maya/include/basicBlendShape.h", "src/maya/src/basicBlendShape.cpp"], "rig": ["src/python/simplexui/commands/rigidAlign.py"], "face": ["src/python/simplexui/commands/correctiveInterface.py", "src/python/simplexui/commands/mayaCorrectiveInterface.py", "src/python/simplexui/interface/__init__.py", "src/python/simplexui/interface/dummyInterface.py", "src/python/simplexui/interface/mayaInterface.py", "src/python/simplexui/interfaceModel.py", "src/python/simplexui/interfaceModelTrees.py"]}

## Prior selected-function review (partial)

{"files": ["src/maya/src/basicBlendShape.cpp"], "finding": "Per-target and per-component weights applied to deformer target deltas. Solver/combo system still needs separate reading."}

This prior partial review may target a different pinned revision; compare hashes before reusing findings.

## Source file: src/maya/src/basicBlendShape.cpp

{
  "path": "src/maya/src/basicBlendShape.cpp",
  "blob_sha": "17f0e88b26bc0079a4b236dc9f3253903d275e1b",
  "sha256": "799ceb550f4cc9c0cb9dba8fdc331207b69275e44f06c376872a25eec5ee1ebf",
  "lines": 136,
  "bytes": 4087,
  "selection_score": 15,
  "keyword_line_counts": {
    "skin": 1,
    "blendshape": 11,
    "gum": 1,
    "deform": 8,
    "rig": 1
  },
  "url": "https://github.com/blurstudio/Simplex/blob/377477f96ea435641b2d2ed2eb6c26cb7bfc96c4/src/maya/src/basicBlendShape.cpp",
  "function_signature_candidates": []
}

```text

//-
// ==========================================================================
// Copyright 2015 Autodesk, Inc.  All rights reserved.
//
// Use of this software is subject to the terms of the Autodesk
// license agreement provided at the time of installation or download,
// or which otherwise accompanies this software in either electronic
// or hard copy form.
// ==========================================================================
//+

//
//  File: basicBlendShape.cpp
//
//  Description:
//      Rudimentary implementation of a blendshape.
//
//      Use this script to create a simple example.
/*      
loadPlugin basicBlendShape;
polyTorus -r 1 -sr 0.5 -tw 0 -sx 50 -sy 50 -ax 0 1 0 -cuv 1 -ch 1;
polyTorus -r 1 -sr 0.5 -tw 0 -sx 50 -sy 50 -ax 0 1 0 -cuv 1 -ch 1;
scale -r 0.5 1 1;
makeIdentity -apply true -t 1 -r 1 -s 1 -n 0 -pn 1;
select -r pTorus1;
deformer -type "basicBlendShape";
blendShape -edit -t pTorus1 0 pTorus2 1.0 basicBlendShape1;
*/

#include "basicBlendShape.h"

#include <maya/MFloatArray.h>
#include <maya/MPoint.h>
#include <maya/MPointArray.h>

#include <maya/MItGeometry.h>
#include <maya/MFnPointArrayData.h>
#include <maya/MFnComponentListData.h>


MTypeId basicBlendShape::id(0x00122706);

void* basicBlendShape::creator() {
    return new basicBlendShape();
}

MStatus basicBlendShape::initialize() {
    return MStatus::kSuccess;
}


MStatus
basicBlendShape::deformData(MDataBlock& block,
					  MDataHandle geomData,
					  unsigned int /*groupId*/,
                      const MMatrix& /*m*/,
                      unsigned int multiIndex)
//
// Method: deform
//
// Description:   Deforms the point with a simple smooth skinning algorithm
//
// Arguments:
//   block      : the datablock of the node
//   geomData   : a handle to the geometry to be deformed
//	 groupId	: the group ID of the geometry to deform
//   m          : matrix to transform the point into world space
//   multiIndex : the index of the geometry that we are deforming
//
//
{
    MStatus returnStatus;

	// get the weights
	//
	MArrayDataHandle weightMH = block.inputArrayValue(weight);
	unsigned int numWeights = weightMH.elementCount();
	MFloatArray weights;
	for (unsigned int w=0; w<numWeights; ++w) {
		weights.append(weightMH.inputValue().asFloat());
		weightMH.next();
	}

	// get the input targets
	// as a point array per weight
	//
	MArrayDataHandle inputTargetMH = block.inputArrayValue(inputTarget);
	returnStatus = inputTargetMH.jumpToElement(multiIndex);
	if (!returnStatus) {
		return returnStatus;
	}
	MDataHandle inputTargetH = inputTargetMH.inputValue();
	MArrayDataHandle inputTargetGroupMH = inputTargetH.child(inputTargetGroup);
	for (unsigned int w=0; w<numWeights; ++w) {
		// inputPointsTarget is computed on pull,
		// so can't just read it out of the datablock
		MPlug plug(thisMObject(), inputPointsTarget);
		plug.selectAncestorLogicalIndex(multiIndex, inputTarget);
		plug.selectAncestorLogicalIndex(w, inputTargetGroup);
		// ignore deformer chains here and just take the first one
		plug.selectAncestorLogicalIndex(6000, inputTargetItem);
		MDGContext context = block.context();
		MObject pointArray = plug.asMObject(context);
		MPointArray pts = MFnPointArrayData(pointArray).array();

		// get the component list
		plug = plug.parent();
		plug = plug.child(inputComponentsTarget);
		MFnComponentListData compList(plug.asMObject(context));
		if (compList.length() == 0) {
			continue;
		}
		MObject comp = compList[0];

		// iterate over the components
		float defWgt = weights[w];
		inputTargetGroupMH.jumpToArrayElement(w);
		MArrayDataHandle targetWeightsMH = inputTargetGroupMH.inputValue().child(targetWeights);
		unsigned int ptIndex = 0;
		MItGeometry iter(geomData, comp, false);
		for (; !iter.isDone(); iter.next(), ++ptIndex) {
			MPoint pt = iter.position();
			unsigned int compIndex = iter.index();
			float wgt = defWgt;
			if (targetWeightsMH.jumpToElement(compIndex)) {
				wgt *= targetWeightsMH.inputValue().asFloat();
			}
			pt += pts[ptIndex] * wgt;
			iter.setPosition(pt);
		}
	}

    return returnStatus;
}




```

## Source file: src/maya/include/basicBlendShape.h

{
  "path": "src/maya/include/basicBlendShape.h",
  "blob_sha": "1665c4d29a075f195f80879c25df7925676f596a",
  "sha256": "2b610b67e610f12f7e2d36b3aa5962eece391623f90c7402869a25cc50463c3b",
  "lines": 53,
  "bytes": 1413,
  "selection_score": 15,
  "keyword_line_counts": {
    "blendshape": 7,
    "deform": 3,
    "rig": 1
  },
  "url": "https://github.com/blurstudio/Simplex/blob/377477f96ea435641b2d2ed2eb6c26cb7bfc96c4/src/maya/include/basicBlendShape.h",
  "function_signature_candidates": []
}

```text

//-
// ==========================================================================
// Copyright 2015 Autodesk, Inc.  All rights reserved.
//
// Use of this software is subject to the terms of the Autodesk
// license agreement provided at the time of installation or download,
// or which otherwise accompanies this software in either electronic
// or hard copy form.
// ==========================================================================
//+

//
//  File: basicBlendShape.cpp
//
//  Description:
//      Rudimentary implementation of a blendshape.
//
//      Use this script to create a simple example.
/*      
loadPlugin basicBlendShape;
polyTorus -r 1 -sr 0.5 -tw 0 -sx 50 -sy 50 -ax 0 1 0 -cuv 1 -ch 1;
polyTorus -r 1 -sr 0.5 -tw 0 -sx 50 -sy 50 -ax 0 1 0 -cuv 1 -ch 1;
scale -r 0.5 1 1;
makeIdentity -apply true -t 1 -r 1 -s 1 -n 0 -pn 1;
select -r pTorus1;
deformer -type "basicBlendShape";
blendShape -edit -t pTorus1 0 pTorus2 1.0 basicBlendShape1;
*/
#pragma once

#include <maya/MTypeId.h> 
#include <maya/MMatrixArray.h>
#include <maya/MPxBlendShape.h> 


class basicBlendShape : public MPxBlendShape {
public:
    static  void*   creator();
    static  MStatus initialize();

    // Deformation function
    //
    virtual MStatus deformData(
		MDataBlock&    block,
		MDataHandle geomData,
		unsigned int groupId,
		const MMatrix& mat,
		unsigned int multiIndex
	);

    static MTypeId id;
};



```

## Source file: src/python/simplexui/interfaceModel.py

{
  "path": "src/python/simplexui/interfaceModel.py",
  "blob_sha": "52c6c526b7f5309614318c66380a330ba0c0e2d8",
  "sha256": "0ffd84817b9dcdcbbfa6da9ea0e61d5531af90c20f5fcd439f05efdab777051b",
  "lines": 655,
  "bytes": 22933,
  "selection_score": 15,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/blurstudio/Simplex/blob/377477f96ea435641b2d2ed2eb6c26cb7bfc96c4/src/python/simplexui/interfaceModel.py",
  "function_signature_candidates": [
    {
      "line": 44,
      "text": "def coerceIndexToType(indexes: list[QModelIndex], typ: type[T]) -> list[QModelIndex]:"
    },
    {
      "line": 82,
      "text": "def coerceIndexToChildType("
    },
    {
      "line": 126,
      "text": "def coerceIndexToParentType("
    },
    {
      "line": 164,
      "text": "def coerceIndexToRoots(indexes: list[QModelIndex]) -> list[QModelIndex]:"
    },
    {
      "line": 216,
      "text": "def simplex(self) -> TreeRootItem | None:"
    },
    {
      "line": 219,
      "text": "def headerData("
    },
    {
      "line": 228,
      "text": "def flags(self, index: QModelIndex) -> Qt.ItemFlag:"
    },
    {
      "line": 248,
      "text": "def setData("
    },
    {
      "line": 287,
      "text": "def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:"
    },
    {
      "line": 291,
      "text": "def sourceModel(self) -> SimplexModel:"
    },
    {
      "line": 294,
      "text": "def indexFromItem(self, item: TreeItem, column: int = 0) -> QModelIndex:"
    },
    {
      "line": 299,
      "text": "def itemFromIndex(self, index: QModelIndex) -> TreeItem | None:"
    },
    {
      "line": 304,
      "text": "def invalidate(self) -> None:"
    },
    {
      "line": 310,
      "text": "def invalidateFilter(self) -> None:"
    },
    {
      "line": 316,
      "text": "def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:"
    },
    {
      "line": 321,
      "text": "def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:"
    },
    {
      "line": 332,
      "text": "def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:"
    },
    {
      "line": 343,
      "text": "def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:"
    },
    {
      "line": 359,
      "text": "def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:"
    },
    {
      "line": 368,
      "text": "def filterString(self) -> str:"
    },
    {
      "line": 372,
      "text": "def filterString(self, val: str) -> None:"
    },
    {
      "line": 384,
      "text": "def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:"
    },
    {
      "line": 398,
      "text": "def matchFilterString(self, itemString: str) -> bool:"
    },
    {
      "line": 406,
      "text": "def matchIsolation(self, itemString: str) -> bool:"
    },
    {
      "line": 411,
      "text": "def checkChildren(self, sourceItem: TreeItem) -> bool:"
    },
    {
      "line": 429,
      "text": "def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:"
    },
    {
      "line": 436,
      "text": "def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:"
    },
    {
      "line": 468,
      "text": "def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:"
    },
    {
      "line": 475,
      "text": "def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:"
    },
    {
      "line": 518,
      "text": "def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:"
    },
    {
      "line": 522,
      "text": "def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:"
    },
    {
      "line": 542,
      "text": "def __init__(self, simplex: Simplex, parent: QWidget | None) -> None:"
    },
    {
      "line": 546,
      "text": "def getItemAppendRow(self, item: TreeItem) -> int:"
    },
    {
      "line": 552,
      "text": "def index("
    },
    {
      "line": 566,
      "text": "def parent(self, index: QModelIndex) -> QModelIndex:"
    },
    {
      "line": 569,
      "text": "def rowCount(self, parent) -> int:"
    },
    {
      "line": 575,
      "text": "def columnCount(self, parent: QModelIndex) -> int:"
    },
    {
      "line": 578,
      "text": "def data(self, index: QModelIndex, role: int) -> Any:"
    },
    {
      "line": 608,
      "text": "def setData(self, index: QModelIndex, value: Any, role: int) -> bool:"
    },
    {
      "line": 638,
      "text": "def flags(self, index: QModelIndex) -> Qt.ItemFlag:"
    },
    {
      "line": 645,
      "text": "def itemFromIndex(self, index: QModelIndex) -> TreeItem:"
    },
    {
      "line": 648,
      "text": "def getItemRow(self, item: Falloff) -> int | None:"
    }
  ]
}

```text

# Copyright 2016, Blur Studio
#
# This file is part of Simplex.
#
# Simplex is free software: you can redistribute it and/or modify
# it under the terms of the GNU Lesser General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Simplex is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Lesser General Public License for more details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with Simplex.  If not, see <http://www.gnu.org/licenses/>.

from __future__ import annotations

import re
from typing import Any, TypeVar, cast

from Qt.QtCore import QModelIndex, QSortFilterProxyModel, Qt
from Qt.QtWidgets import QWidget

from .items import (
    Combo,
    ComboPair,
    Falloff,
    Group,
    ProgPair,
    Progression,
    Simplex,
    Slider,
    Traversal,
    TravPair,
)
from .items.treeItem import AdapterModel, TreeItem, TreeRootItem

T = TypeVar('T', bound=TreeItem)


# Hierarchy Helpers
def coerceIndexToType(indexes: list[QModelIndex], typ: type[T]) -> list[QModelIndex]:
    """Get a list of indices of a specific type based on a given index list
    Items containing parents of the type fall down to their children
    Items containing children of the type climb up to their parents

    Parameters
    ----------
    indexes : [QModelIndex, ...]
        A list of indexes to coerce
    typ : Type
        The type to coerce to

    Returns
    -------
    [QModelIndex, ...]
        The coerced list
    """
    targetDepth = typ.classDepth

    children = []
    parents = []
    out = []
    for idx in indexes:
        item = idx.model().itemFromIndex(idx)
        depth = item.classDepth
        if depth < targetDepth:
            parents.append(idx)
        elif depth > targetDepth:
            children.append(idx)
        else:
            out.append(idx)

    out.extend(coerceIndexToChildType(parents, typ))
    out.extend(coerceIndexToParentType(children, typ))
    out = list(set(out))
    return out


def coerceIndexToChildType(
    indexes: list[QModelIndex], typ: type[T]
) -> list[QModelIndex]:
    """Get a list of indices of a specific type based on a given index list
        Lists containing parents of the type fall down to their children

    Parameters
    ----------
    indexes : [QModelIndex, ...]
        A list of indexes to coerce
    typ : Type
        The type to coerce to

    Returns
    -------
    [QModelIndex, ...]
        The coerced list
    """
    targetDepth = typ.classDepth
    out = []

    for idx in indexes:
        model = idx.model()
        item = idx.model().itemFromIndex(idx)
        if item.classDepth < targetDepth:
            # Too high up, grab children
            queue = [idx]
            depthIdxs = []
            while queue:
                checkIdx = queue.pop()
                checkItem = checkIdx.model().itemFromIndex(checkIdx)
                if checkItem.classDepth < targetDepth:
                    for row in range(model.rowCount(checkIdx)):
                        queue.append(model.index(row, 0, checkIdx))
                elif checkItem.classDepth == targetDepth:
                    depthIdxs.append(checkIdx)
            out.extend(depthIdxs)
        elif item.classDepth == targetDepth:
            out.append(idx)

    out = list(set(out))
    return out


def coerceIndexToParentType(
    indexes: list[QModelIndex], typ: type[T]
) -> list[QModelIndex]:
    """Get a list of indices of a specific type based on a given index list
        Lists containing children of the type climb up to their parents

    Parameters
    ----------
    indexes : [QModelIndex, ...]
        A list of indexes to coerce
    typ : Type
        The type to coerce to

    Returns
    -------
    [QModelIndex, ...]
        The coerced list
    """
    targetDepth = typ.classDepth
    out = []
    for idx in indexes:
        item = idx.model().itemFromIndex(idx)
        depth = item.classDepth
        if depth > targetDepth:
            parIdx = idx
            parItem = parIdx.model().itemFromIndex(parIdx)
            while parItem.classDepth > targetDepth:
                parIdx = parIdx.parent()
                parItem = parIdx.model().itemFromIndex(parIdx)
            if parItem.classDepth == targetDepth:
                out.append(parIdx)
        elif depth == targetDepth:
            out.append(idx)

    out = list(set(out))
    return out


def coerceIndexToRoots(indexes: list[QModelIndex]) -> list[QModelIndex]:
    """Get the topmost indexes for each brach in the hierarchy

    Parameters
    ----------
    indexes : [QModelIndex, ...]
        A list of indexes to coerce

    Returns
    -------
    [QModelIndex, ...]
        The coerced list
    """
    indexes = [i for i in indexes if i.column() == 0]
    indexes = sorted(
        indexes, key=lambda x: x.model().itemFromIndex(x).classDepth, reverse=True
    )
    # Check each item to see if any of it's ancestors
    # are in the selection list.  If not, it's a root
    roots = []
    for idx in indexes:
        par = idx.parent()
        while par.isValid():
            if par in indexes:
                break
            par = par.parent()
        else:
            roots.append(idx)

    return roots


# BASE MODEL
class SimplexModel(AdapterModel):
    """The base model for all interaction with a simplex system.
    All ui interactions with a simplex system must go through this model
    Any special requirements, or reorganizations of the trees will only
    be implemented as proxy models.

    There will be little documentation for this class, as all methods
    are virtual overrides of the underlying Qt class

    Parameters
    ----------
    simplex : Simplex
        The Simplex system for this model
    parent : QObject
        The parent for this model

    """

    @property
    def simplex(self) -> TreeRootItem | None:
        return self._rootItem

    def headerData(
        self, section: int, orientation: Qt.Orientation, role: int
    ) -> str | None:
        if orientation == Qt.Orientation.Horizontal:
            if role == Qt.ItemDataRole.DisplayRole:
                sects = ("Items", "Slide", "Value")
                return sects[section]
        return None

    def flags(self, index: QModelIndex) -> Qt.ItemFlag:
        if not index.isValid():
            return Qt.ItemFlag.ItemIsEnabled
        if index.column() == 0:
            item = index.internalPointer()
            if isinstance(item, (Slider, Combo, Traversal)):
                return (
                    Qt.ItemFlag.ItemIsEnabled
                    | Qt.ItemFlag.ItemIsSelectable
                    | Qt.ItemFlag.ItemIsEditable
                    | Qt.ItemFlag.ItemIsUserCheckable
                )
            elif isinstance(item, Progression):
                return Qt.ItemFlag.ItemIsEnabled | Qt.ItemFlag.ItemIsSelectable
        return (
            Qt.ItemFlag.ItemIsEnabled
            | Qt.ItemFlag.ItemIsSelectable
            | Qt.ItemFlag.ItemIsEditable
        )

    def setData(
        self, index: QModelIndex, value: Any, role: int = Qt.ItemDataRole.EditRole
    ) -> bool:
        if not index.isValid():
            return False
        if role == Qt.ItemDataRole.CheckStateRole:
            item = index.internalPointer()
            if index.column() == 0:
                if isinstance(item, (Slider, Combo, Traversal)):
                    item.enabled = value == Qt.CheckState.Checked.value
                    return True
        elif role == Qt.ItemDataRole.EditRole:
            item = index.internalPointer()
            if index.column() == 0:
                if isinstance(item, (Group, Slider, Combo, Traversal, ProgPair)):
                    item.name = value
                    return True

            elif index.column() == 1:
                if isinstance(item, Slider):
                    item.value = value
                elif isinstance(item, ComboPair):
                    item.value = value
                elif isinstance(item, TravPair):
                    item.value = value

            elif index.column() == 2:
                if isinstance(item, ProgPair):
                    item.value = value
        return False


# VIEW MODELS
class BaseProxyModel(QSortFilterProxyModel):
    """Holds the common item/index translation code for my filter models
    Again, This is just a concrete implementation of a Qt base class, so
    documentation will be lacking
    """

    def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setSourceModel(model)

    def sourceModel(self) -> SimplexModel:
        return cast(SimplexModel, super().sourceModel())

    def indexFromItem(self, item: TreeItem, column: int = 0) -> QModelIndex:
        sourceModel = self.sourceModel()
        sourceIndex = sourceModel.indexFromItem(item, column)
        return self.mapFromSource(sourceIndex)

    def itemFromIndex(self, index: QModelIndex) -> TreeItem | None:
        sourceModel = self.sourceModel()
        sIndex = self.mapToSource(index)
        return sourceModel.itemFromIndex(sIndex)

    def invalidate(self) -> None:
        source = self.sourceModel()
        if isinstance(source, QSortFilterProxyModel):
            source.invalidate()
        super().invalidate()

    def invalidateFilter(self) -> None:
        source = self.sourceModel()
        if isinstance(source, QSortFilterProxyModel):
            source.invalidateFilter()
        super().invalidateFilter()

    def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:
        return True


class SliderModel(BaseProxyModel):
    def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:
        sourceIndex = self.sourceModel().index(sourceRow, 0, sourceParent)
        if sourceIndex.isValid():
            item = self.sourceModel().itemFromIndex(sourceIndex)
            if isinstance(item, Group):
                if item.groupType is not Slider:
                    return False
        return super().filterAcceptsRow(sourceRow, sourceParent)


class ComboModel(BaseProxyModel):
    def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:
        sourceIndex = self.sourceModel().index(sourceRow, 0, sourceParent)
        if sourceIndex.isValid():
            item = self.sourceModel().itemFromIndex(sourceIndex)
            if isinstance(item, Group):
                if item.groupType is not Combo:
                    return False
        return super().filterAcceptsRow(sourceRow, sourceParent)


class TraversalModel(BaseProxyModel):
    def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:
        sourceIndex = self.sourceModel().index(sourceRow, 0, sourceParent)
        if sourceIndex.isValid():
            item = self.sourceModel().itemFromIndex(sourceIndex)
            if isinstance(item, Group):
                if item.groupType is not Traversal:
                    return False
        return super().filterAcceptsRow(sourceRow, sourceParent)


# FILTER MODELS
class SimplexFilterModel(BaseProxyModel):
    """Filter a model based off of a given string
    Set the `filterString` object property to filter the model
    """

    def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:
        super().__init__(model, parent)
        self.setSourceModel(model)
        self.filterShapes: bool = True
        self._filterString: list[str] = []
        self._filterReg: list[re.Pattern[str]] = []
        self.isolateList: list[str] = []

    @property
    def filterString(self) -> str:
        return " ".join(self._filterString)

    @filterString.setter
    def filterString(self, val: str) -> None:
        self._filterString = val.split()

        self._filterReg = []
        for sp in self._filterString:
            if sp[0] == "*":
                rex = re.compile(sp, flags=re.I)
                self._filterReg.append(rex)
            else:
                rex = re.compile(".*?".join(sp), flags=re.I)
                self._filterReg.append(rex)

    def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:
        column = 0  # always sort by the first column #column = self.filterKeyColumn()
        sourceIndex = self.sourceModel().index(sourceRow, column, sourceParent)
        if sourceIndex.isValid():
            if self.filterString or self.isolateList:
                sourceItem = self.sourceModel().itemFromIndex(sourceIndex)
                if isinstance(
                    sourceItem, (ProgPair, Slider, Combo, ComboPair, Progression)
                ):
                    if not self.checkChildren(sourceItem):
                        return False

        return super().filterAcceptsRow(sourceRow, sourceParent)

    def matchFilterString(self, itemString: str) -> bool:
        if not self._filterString:
            return True
        for reg in self._filterReg:
            if reg.search(itemString):
                return True
        return False

    def matchIsolation(self, itemString: str) -> bool:
        if self.isolateList:
            return itemString in self.isolateList
        return True

    def checkChildren(self, sourceItem: TreeItem) -> bool:
        if hasattr(sourceItem, 'name'):
            itemString = sourceItem.name  # type: ignore
            if self.matchFilterString(itemString) and self.matchIsolation(itemString):
                return True

        sourceModel = self.sourceModel().sourceModel()
        for row in range(sourceModel.getItemRowCount(sourceItem)):
            childItem = sourceModel.getChildItem(sourceItem, row)
            if childItem is not None:
                return self.checkChildren(childItem)

        return False


class SliderFilterModel(SimplexFilterModel):
    """Hide single shapes under a slider"""

    def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:
        super().__init__(model, parent)
        self.requires: list[Combo] = []
        self.filterRequiresAny: bool = False
        self.filterRequiresAll: bool = False
        self.doFilter: bool = True

    def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:
        # always sort by the first column #column = self.filterKeyColumn()
        column = 0
        sourceIndex = self.sourceModel().index(sourceRow, column, sourceParent)
        if sourceIndex.isValid():
            data = self.sourceModel().itemFromIndex(sourceIndex)
            if self.doFilter:
                if isinstance(data, ProgPair):
                    if data.prog is None:
                        return False
                    elif len(data.prog.pairs) <= 2:
                        return False
                    elif data.shape.isRest:
                        return False

            if (self.filterRequiresAny or self.filterRequiresAll) and self.requires:
                # Ignore items that aren't part of the required combos, if requested
                if isinstance(data, Slider):
                    sliGroups = [[i.slider for i in c.pairs] for c in self.requires]
                    if self.filterRequiresAny:
                        if not any(data in s for s in sliGroups):
                            return False
                    elif self.filterRequiresAll:
                        if not all(data in s for s in sliGroups):
                            return False

        return super().filterAcceptsRow(sourceRow, sourceParent)


class ComboFilterModel(SimplexFilterModel):
    """Filter by slider when Show Dependent Combos is checked"""

    def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:
        super().__init__(model, parent)
        self.requires: list[Slider] = []
        self.filterRequiresAll: bool = False
        self.filterRequiresAny: bool = False
        self.filterRequiresOnly: bool = False

    def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:
        # always sort by the first column #column = self.filterKeyColumn()
        column = 0
        sourceIndex = self.sourceModel().index(sourceRow, column, sourceParent)
        if sourceIndex.isValid():
            data = self.sourceModel().itemFromIndex(sourceIndex)
            if self.filterShapes:
                # ignore the SHAPE par if there's nothing under there
                if isinstance(data, Progression):
                    if len(data.pairs) <= 2:
                        return False
                # Ignore shape things if requested
                if isinstance(data, ProgPair):
                    if data.prog is None:
                        return False
                    elif len(data.prog.pairs) <= 2:
                        return False
                    elif data.shape.isRest:
                        return False
            if (
                self.filterRequiresAny
                or self.filterRequiresAll
                or self.filterRequiresOnly
            ) and self.requires:
                # Ignore items that don't use the required sliders if requested
                if isinstance(data, Combo):
                    sliders = [i.slider for i in data.pairs]
                    if self.filterRequiresAll:
                        if not all(r in sliders for r in self.requires):
                            return False
                    elif self.filterRequiresAny:
                        if not any(r in sliders for r in self.requires):
                            return False
                    elif self.filterRequiresOnly:
                        if not all(r in self.requires for r in sliders):
                            return False

        return super().filterAcceptsRow(sourceRow, sourceParent)


class TraversalFilterModel(SimplexFilterModel):
    """Hide single shapes under a slider"""

    def __init__(self, model: SimplexModel, parent: QWidget | None = None) -> None:
        super().__init__(model, parent)
        self.doFilter = True

    def filterAcceptsRow(self, sourceRow: int, sourceParent: QModelIndex) -> bool:
        column = 0  # always sort by the first column #column = self.filterKeyColumn()
        sourceIndex = self.sourceModel().index(sourceRow, column, sourceParent)
        if sourceIndex.isValid():
            if self.doFilter:
                data = self.sourceModel().itemFromIndex(sourceIndex)
                if isinstance(data, ProgPair):
                    if data.prog is None:
                        return False
                    elif len(data.prog.pairs) <= 2:
                        return False
                    elif data.shape.isRest:
                        return False

        return super().filterAcceptsRow(sourceRow, sourceParent)


class FalloffDataModel(AdapterModel):
    """A model for displaying the data of Falloff objects"""

    def __init__(self, simplex: Simplex, parent: QWidget | None) -> None:
        super().__init__(simplex, parent)
        self.simplex = simplex

    def getItemAppendRow(self, item: TreeItem) -> int:
        try:
            return len(self.simplex.falloffs)
        except AttributeError:
            return 0

    def index(
        self, row: int, column: int = 0, parIndex: QModelIndex | None = None
    ) -> QModelIndex:
        parIndex = QModelIndex() if parIndex is None else parIndex
        if row < 0:
            return QModelIndex()
        try:
            falloff = self.simplex.falloffs[row]
        except IndexError:
            return QModelIndex()
        except AttributeError:
            return QModelIndex()
        return self.createIndex(row, column, falloff)

    def parent(self, index: QModelIndex) -> QModelIndex:
        return QModelIndex()

    def rowCount(self, parent) -> int:
        try:
            return len(self.simplex.falloffs)
        except AttributeError:
            return 0

    def columnCount(self, parent: QModelIndex) -> int:
        return 8

    def data(self, index: QModelIndex, role: int) -> Any:
        if not index.isValid():
            return None
        falloff = index.internalPointer()
        if not falloff:
            return None

        if role in (Qt.ItemDataRole.DisplayRole, Qt.ItemDataRole.EditRole):
            if index.column() == 0:
                return falloff.name
            elif index.column() == 1:
                st = falloff.splitType
                sti = ("planar", "map").index(st.lower())
                return sti
            elif index.column() == 2:
                ax = falloff.axis
                axi = "xyz".index(ax.lower())
                return axi
            elif index.column() == 3:
                return falloff.maxVal
            elif index.column() == 4:
                return falloff.maxHandle
            elif index.column() == 5:
                return falloff.minHandle
            elif index.column() == 6:
                return falloff.minVal
            elif index.column() == 7:
                return falloff.mapName
        return None

    def setData(self, index: QModelIndex, value: Any, role: int) -> bool:
        if not index.isValid():
            return False
        falloff = index.internalPointer()
        if not falloff:
            return False
        if role == Qt.ItemDataRole.EditRole:
            if index.column() == 0:
                falloff.name = value
            elif index.column() == 1:
                if value in [0, 1]:
                    value = ("planar", "map")[value]
                falloff.splitType = value
            elif index.column() == 2:
                if value in [0, 1, 2]:
                    value = "XYZ"[value]
                falloff.axis = value
            elif index.column() == 3:
                falloff.maxVal = value
            elif index.column() == 4:
                falloff.maxHandle = value
            elif index.column() == 5:
                falloff.minHandle = value
            elif index.column() == 6:
                falloff.minVal = value
            elif index.column() == 7:
                falloff.mapName = value
            return True
        return False

    def flags(self, index: QModelIndex) -> Qt.ItemFlag:
        return (
            Qt.ItemFlag.ItemIsEnabled
            | Qt.ItemFlag.ItemIsEditable
            | Qt.ItemFlag.ItemIsSelectable
        )

    def itemFromIndex(self, index: QModelIndex) -> TreeItem:
        return index.internalPointer()

    def getItemRow(self, item: Falloff) -> int | None:
        try:
            idx = self.simplex.falloffs.index(item)
        except ValueError:
            return None
        except AttributeError:
            return None
        return idx


```

## Source file: src/python/simplexui/commands/rigidAlign.py

{
  "path": "src/python/simplexui/commands/rigidAlign.py",
  "blob_sha": "e9a10a4a3f1bdc79d4f3c09e4ddff4b446e299a3",
  "sha256": "c637cdb098ebe94ea1052cdcbad123a3d68d8a1146de24e94e9c99a67e608209",
  "lines": 97,
  "bytes": 3133,
  "selection_score": 15,
  "keyword_line_counts": {
    "eye": 2,
    "rig": 3
  },
  "url": "https://github.com/blurstudio/Simplex/blob/377477f96ea435641b2d2ed2eb6c26cb7bfc96c4/src/python/simplexui/commands/rigidAlign.py",
  "function_signature_candidates": [
    {
      "line": 23,
      "text": "def rigidAlign(P, Q, iters: int = 10):"
    }
  ]
}

```text

# Copyright 2016, Blur Studio
#
# This file is part of Simplex.
#
# Simplex is free software: you can redistribute it and/or modify
# it under the terms of the GNU Lesser General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Simplex is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Lesser General Public License for more details.
#
# You should have received a copy of the GNU Lesser General Public License
# along with Simplex.  If not, see <http://www.gnu.org/licenses/>.

from __future__ import annotations

import numpy as np


def rigidAlign(P, Q, iters: int = 10):
    """Rigidly align meshes with matching vert order by a least-squares error.
    Uses a variation of an algorithm by Umeyama
    Relevant links:
    * https://gist.github.com/nh2/bc4e2981b0e213fefd4aaa33edfb3893 (this code)
    * http://stackoverflow.com/a/32244818/263061 (solution with scale)

    Parameters
    ----------
    P : np.array
        Static set of points
    Q : np.array
        Points to align with non-uniform scale
    iters : int
        The number of iterations (Defaults to 10)

    Returns
    -------
    : np.array
        The 4x4 transformation matrix that most closely aligns Q to P
    """
    assert P.shape == Q.shape

    n, dim = P.shape
    assert dim == 3

    if iters <= 1:
        raise ValueError("Must run at least 1 iteration")

    # Get the centroid of each object
    Qm = Q.mean(axis=0)
    Pm = P.mean(axis=0)

    # Subtract out the centroid to get the basic aligned mesh
    cP = P - Pm  # centeredP
    cQRaw = Q - Qm  # centeredQ

    cQ = cQRaw.copy()
    cumulation = np.eye(3)  # build an accumulator for the rotation

    # Here, we find an approximate rotation and scaling, but only
    # keep track of the accumulated rotations.
    # Then we apply the non-uniform scale by comparing bounding boxes
    # This way we don't get any shear in our matrix, and we relatively
    # quickly walk our way towards a minimum
    for _ in range(iters):
        # Magic?
        C = np.dot(cP.T, cQ) / n
        V, S, W = np.linalg.svd(C)

        # Handle negative scaling
        d = (np.linalg.det(V) * np.linalg.det(W)) < 0.0
        if d:
            S[-1] = -S[-1]
            V[:, -1] = -V[:, -1]

        # build the rotation matrix for this iteration
        # and add it to the accumulation
        R = np.dot(V, W)
        cumulation = np.dot(cumulation, R.T)

        # Now apply the accumulated rotation to the raw point positions
        # Then grab the non-uniform scaling from the bounding box
        # And set up cQ for the next iteration
        cQ = np.dot(cQRaw, cumulation)
        sf = (cP.max(axis=0) - cP.min(axis=0)) / (cQ.max(axis=0) - cQ.min(axis=0))
        cQ = cQ * sf

    # Build the final transformation
    csf = cumulation * sf
    tran = Pm - Qm.dot(csf)
    outMat = np.eye(4)
    outMat[:3, :3] = csf
    outMat[3, :3] = tran
    return outMat


```
