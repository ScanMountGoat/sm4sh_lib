# sm4sh_shader
sm4sh_shader is a CLI tool for analyzing in game shaders and creating the shader database.

The current implementation uses the shader graph analysis and query logic from [xc3_shader](https://github.com/ScanMountGoat/xc3_lib/tree/main/xc3_shader).

## Usage
Creating a shader database requires running all the commands in the appropriate order.

```shell
sm4sh_shader dump-shaders "data/shader/texas_cross.nsh" "shader dump" "gfd-tool.exe"
sm4sh_shader match-shaders "shader_ids.txt" "shader_cemu_names.txt" "data/shader/texas_cross.nsh" "shader_ids_shaders.txt"
sm4sh_shader annotate-shaders "shader dump"
sm4sh_shader shader-database "shader_ids_shaders.txt" "shader dump" "shaders.bin"
```

The Cemu names for each shader were dumped from RenderDoc using the following Python script. Enable Debug > Dump > Shaders prior to launching a game. Matching up the shader IDs to shader binaries requires a model.nud with one mesh draw call for each shader ID in ascending order.

```python
def processActions(controller, d):
    # Find the appropriate draws recursively.
    # Each shader ID mesh is a cube with 36 indices.
    # Some of these draws will need to be filtered out manually later.
    if d.numIndices == 36:
        # Move to that draw
        controller.SetFrameEvent(d.eventId, True)
        processDraw(controller)

    for c in d.children:
        processActions(controller, c)


def processDraw(controller):
    state = controller.GetPipelineState()

    vs = state.GetShaderReflection(renderdoc.ShaderStage.Vertex)
    ps = state.GetShaderReflection(renderdoc.ShaderStage.Pixel)

    vert = pyrenderdoc.GetResourceName(vs.resourceId)
    pixel = pyrenderdoc.GetResourceName(ps.resourceId)
    print(f"{vert}, {pixel}")


def sampleCode(controller):
    for d in controller.GetRootActions():
        processActions(controller, d)


pyrenderdoc.Replay().BlockInvoke(sampleCode)
```
