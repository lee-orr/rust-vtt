Baking Process:

CPU
- Gather list of changed/added/removed objects
- Determine bounding boxes for changed regions
- Determine zones
- Provide brushes, zones, changed bounding boxes & updated camera locations to GPU
- Provide initial update requests for root & up to N potentially other updated locations, up to a certain depth, prioritizing changed areas

GPU
- Generate new hash tree - migrate all nodes to new tree, adjusting hashes based on new zone locations (updated location buffer, cache)
- Move any nodes that have been accessed in the last frame to the front of the cache (cache)
- If there is an update buffer from the previous frame, fill in any available space in the new update buffer with requests from there (new request buffer, update buffer from last frame, dispatch buffer)
- Run initial update bake
    - check center of zone - if has surface with radius make a request and store it's ID & info in secondary bake request buffer, and set up dispatch buffer (brushes, secondary bake buffer, cache)
    - for any nodes within that get a secondary request, bake the block in the texture using secondary bake buffer info (secondary bake buffer, brushes, zones, texture)
- Run low res raycast (cache, request buffer, texture(read))
    - during raycase, store requests for nodes that don't exist at the necessary LOD, going up to the existing LOD (so if I need a node at LOD 2 but it only exists at 0, it'll request both 1 & 2) in the main update buffer
- Run update bake, same as before
- Run full res raycast (cache, request buffer, lights)
    - store new request in main update buffer

GPU Cache (storage buffers required: 2 (hash + circular) always, 1 (texture) on write only)
- Initialize circular buffer with 3d texture addresses for all available nodes - because the nodes will move in the buffer, those addresses need to remain with the node and not tied to the index directly (Once on start)
- Circular buffer + hash tree (pointing to the node in the circular buffer) & 3D texture addressed through transforming the index from the circular buffer
- When a node is requested from the cache:
    - Check the hash table:
        - if the node exists:
            - atomically update the nodes last use with the current frame number (with the ability to circle back if needed)
            - return the node
        - if the node doesn't exist:
            - append the node to the request buffer, check the node 1 LOD lower. Repeat until a hit is found
- When a node is removed from the cache:
    - atomically set it's last use to: `current frame - 100`
    - remove it from the hash table
- When a node is added to the cache:
    - atomically Insert the key to the hash table
    - atomically grab the last index in the circular buffer
    - set hash value to the index
    - reset the node, and mark it as `current frame`
- When a node is updated:
    - grab the node ID (as above)
    - update the baked data
    - mark as `current frame`
