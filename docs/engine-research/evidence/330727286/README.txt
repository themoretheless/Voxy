![BTLogo](https://github.com/user-attachments/assets/f1890550-000c-4289-99aa-1ede37d08746)

# GodotBT: Behavior Tree Framework for Godot 4.x

A flexible, extensible behavior tree framework for Godot 4.x that makes it easy to create complex AI behaviors for your games.

## Overview

GodotBT provides a complete behavior tree implementation for the Godot Engine, allowing you to create sophisticated AI behaviors with a clear, modular structure. The framework follows behavior tree design principles with a focus on reusability and separation of concerns.

## Key Features

- **Complete Behavior Tree Implementation**: Core composite nodes, decorators, services, tasks, and conditions
- **Reusable Behavior Trees**: Define a behavior tree once and use it across multiple agents with their own contexts
- **Blackboard System**: Flexible data sharing between nodes with typed accessors and change detection
- **Reactive Conditions**: Trigger behavior changes based on world events with configurable abort scopes
- **Service System**: Run periodic background tasks with customizable frequency
- **Context Separation**: Each agent maintains its own execution context with the shared tree
- **Utility Components**: Helpers like BTTargetKey for simplified movement and targeting operations
- **Example Implementation**: Includes a complete demo with enemies that can patrol, chase the player, and react to events

## Core Components

### Composite Nodes

- **BTSelector**: Runs child nodes until one succeeds or all fail
- **BTSequence**: Runs child nodes until one fails or all succeed
- **BTParallel**: Runs all child nodes simultaneously
- **BTRandomSelector/BTRandomSequence**: Random execution order versions of selector and sequence

### Decorators

- **BTInverter**: Inverts the result of a node
- **BTRepeater**: Repeats a node a specified number of times
- **BTRepeatUntil**: Repeats a node until it returns a specific result
- **BTAlwaysReturn**: Forces a node to return a specific result

### Conditions

- **BTBlackboardBasedCondition**: Condition based on blackboard values
- **BTReactiveCondition**: Condition that can trigger aborts
- **BTCheckNonZeroBBEntry**: Checks if a blackboard value is non-zero/non-empty
- **BTCompareBBEntries**: Compares two blackboard entries

### Services

- **BTService**: Base class for services that run periodically
- Custom services in examples like **BTPlayerDetector**, **BTPatrolPathFollower**

### Tasks

- **BTTask**: Base class for leaf nodes that perform actions
- **BTWait**: Wait for a specified amount of time
- Example implementations for movement, path finding, and more

## Installation

1. Clone or download this repository
2. Copy the `addons/godot_bt` folder into your project's `addons` folder
3. Enable the plugin in Project Settings > Plugins

## Basic Usage

### Creating a Behavior Tree

1. Create a new scene with a root node of type `BehaviorTree`
2. Add a `BTSelector` or `BTSequence` as the first child
3. Build your behavior tree by adding more composite nodes and tasks
4. Save the scene

### Using the Behavior Tree with Multiple Agents

```gdscript
# Level.gd - Sets up the behavior tree and passes it to agents
extends Node2D

func _ready():
    # Find all agents and set their behavior trees
    for enemy in get_tree().get_nodes_in_group("enemies"):
        enemy.run_behavior_tree()
```

```gdscript
# Enemy.gd - Each agent handles its own context
extends CharacterBody2D

@export var bt: BehaviorTree
@export var patrol_path: Node2D

var ctx: BTContext

# Called by the level or manager
func run_behavior_tree() -> void:
    if not is_instance_valid(bt):
        return

    ctx = bt.create_context(self, Blackboard.new())

    # Set up blackboard values for this agent
    if patrol_path:
        var patrol_points: Array = []
        for patrol_pt in patrol_path.get_children():
            patrol_points.append(patrol_pt.global_position)
        ctx.blackboard.set_value("patrol_points", patrol_points)

func _physics_process(delta: float) -> void:
    if is_instance_valid(bt) and ctx:
        bt.tick(ctx, delta)
```

## Example Implementation

The included example demonstrates:

1. **Enemy Patrol System**: Enemies follow patrol paths or look in random directions
2. **Player Detection**: A service that detects the player within a specified range
3. **Chase Behavior**: Enemies chase the player when detected
4. **Navigation**: Path finding to navigate around obstacles

To see the example in action, open the `example/TestLevel.tscn` scene.

## Extending the Framework

The GodotBT framework is designed to be highly extensible. You can:

### Create Custom Tasks

```gdscript
class_name MyCustomTask extends BTTask

@export var _some_parameter: float = 1.0

func _tick(ctx: BTContext) -> BTResult:
    # Your implementation here
    if some_condition:
        return BTResult.SUCCESS
    else:
        return BTResult.RUNNING
```

### Create Custom Conditions

```gdscript
class_name MyCustomCondition extends BTCondition

func _tick(ctx: BTContext) -> bool:
    # Your implementation here
    return some_condition_check
```

### Create Custom Services

```gdscript
class_name MyCustomService extends BTService

func _tick(ctx: BTContext) -> void:
    # Your implementation here
    ctx.blackboard.set_value("some_key", calculate_some_value())
```

## License

This project is available under the MIT License.

## Contributing

Contributions are welcome! Feel free to submit pull requests or open issues for bugs and feature requests.
