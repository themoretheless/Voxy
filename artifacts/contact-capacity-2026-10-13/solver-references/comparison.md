# Solver source comparison

Pinned primary source files and hashes are recorded in sources.json. This is source inspection of two repositories, not a 500-repository completion claim or an implementation/performance qualification.

## TressFX

Source: https://raw.githubusercontent.com/GPUOpen-Effects/TressFX/ba0bdacdfb964e38522fda812bf23169bc5fa603/src/Shaders/TressFXSimulation.hlsl

Inspected entry points: IntegrationAndGlobalShapeConstraints (line610), VelocityShockPropagation (740), LocalShapeConstraints (774), LengthConstriantsWindAndCollision (876). The last kernel performs length iterations between workgroup barriers (925-937). Particle positions remain in GPU storage between simulation stages. This illustrates grouping local iterative work inside a device kernel instead of reading back each local correction.

It is not a drop-in replacement for Voxy's coupled unilateral contact problem or its original f64 admission. Importing its fixed iteration count as a success criterion would change our completion semantics. Velocity shock propagation and local shape controls must not silently replace calibrated material laws.

## PositionBasedDynamics

Source: https://raw.githubusercontent.com/InteractiveComputerGraphics/PositionBasedDynamics/abd45249c2763cdd07497c314344f7749042e199/PositionBasedDynamics/PositionBasedElasticRods.cpp

Inspected direct stiff-rod factor/solve implementation (735-984) and caller (1110-1131). The tree stores small block factors; factorization and forward/backward solves are separate. Stored LDLT factors are reused during the solve. This suggests keeping structural factors resident across right-hand sides. It does not prove that a graph with inter-strand contact cycles can be treated as a tree.

## Current Voxy comparison and next experiment

Voxy already has frozen structural factors, independent-island batching, complete resident active-set kernels, and original-coordinate admission. The full-model profile still shows thousands of separate outer calls. Repeating existing factor reuse is not a new optimization.

Next measure the counts and causes of successive outer physical/Newton calls for an unchanged frozen structural state, separately from equality-kernel iterations. A candidate GPU-resident outer iteration must read all original current constraints, retain coupled owners, and return only after the original residual/complementarity gate or an explicit failure. It must not reinterpret iteration exhaustion as convergence. First reproduce an exact captured problematic owner before changing this orchestration.

## RAG coverage

Index query and compiled-wiki query did not yield a relevant solver implementation. Raw hybrid retrieval timed out; lexical hair/contact search returned mainly facial-rendering/animation references. A TressFX lexical query returned no hits. These results do not prove the corpus lacks other useful documents. Archival Voxy wiki bf4d16b9-530d-4a42-a9f6-065f4c9472c4 describes historical skinning support; it is not current full-groom qualification.
