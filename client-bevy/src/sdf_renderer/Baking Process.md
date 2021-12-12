Baking Process:

CPU
- Gather list of used objects
- Determine removed objects
- Determine new objects
- Place objects in zones around camera
GPU
- If removed objects:
    - Mark trees to clean in buffers based on removed objects
- If adding new objects:
    - determine available pages in cache
    - Place L0 of new objects in buffer
    - Bake L0
    - For each additional level of detail:
        - determine bricks of Ln to generate
        - Place bricks in buffer
        - Bake bricks

Render Process:

Fragment
- March ray through zones
- When in a zone with objects, check points presence in oct-trees and determine marching increments (min)
- If hit
    - for now, return color + normal & derpth
    - for later, march to lights & determine color for the pixel & depth
- If miss, move on to next zone or mark as infinite depth & transparent.
