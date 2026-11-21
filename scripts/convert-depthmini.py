"""Convert the pinned DepthMini Core ML graph for ONNX Runtime (opset 11).
Run with coremltools 9.0, numpy and onnx 1.19.1 against the depth-model-1
.mlpackage. Constants are unpacked from the original palette, never retrained.
"""
import argparse
import hashlib
from pathlib import Path

import coremltools as ct
import numpy as np
import onnx
from coremltools.converters.mil.frontend.milproto.load import load
from onnx import TensorProto, helper, numpy_helper

FILES = {
    "Manifest.json": "cfaa07eae081a343ff0e011c2875206d832335e55073252c29c79f73fd938929",
    "Data/com.apple.CoreML/model.mlmodel": "c9e2f4f1e44409a2b9f5fb0b072ba41fb42d03a555c333f635f5162676feb744",
    "Data/com.apple.CoreML/weights/weight.bin": "63d066f1aefc362f2cc0805a624e08df13b6cff3be45dc4307b20a531a2a9f86",
}


def convert(package, output):
    for path, sha in FILES.items():
        if hashlib.sha256((package / path).read_bytes()).hexdigest() != sha:
            raise ValueError(f"Unexpected upstream file: {path}")
    spec = ct.models.utils.load_spec(str(package))
    fn = load(spec, spec.specificationVersion, str(package / "Data/com.apple.CoreML/weights")).functions["main"]
    nodes, constants = [], {}
    def constant(name, value):
        if name not in constants:
            constants[name] = numpy_helper.from_array(np.asarray(value, dtype=np.float32), name=name)
        return name

    def tensor(var):
        if var.op is not None and var.op.op_type in ("const", "constexpr_lut_to_dense"):
            if var.name not in constants:
                data = var.val if var.op.op_type == "const" else var.op.materialized_val_inference()
                constant(var.name, data)
        return var.name

    for op in fn.operations:
        kind = op.op_type
        if kind in ("const", "constexpr_lut_to_dense"):
            continue
        out = op.outputs[0].name
        def emit(name, inputs, **attrs):
            nodes.append(helper.make_node(name, inputs, [out], name=out, **attrs))

        if kind == "cast":
            emit("Identity", [tensor(op.x)])
        elif kind in ("mul", "add", "sub"):
            emit(kind.capitalize(), [tensor(op.x), tensor(op.y)])
        elif kind == "relu":
            emit("Relu", [tensor(op.x)])
        elif kind == "thresholded_relu":
            # MIL retains x at equality; ONNX ThresholdedRelu uses strict greater.
            lower = out + "_below"
            nodes.append(helper.make_node("Less", [tensor(op.x), tensor(op.alpha)], [lower]))
            emit("Where", [lower, constant("depthmini_zero", 0.), tensor(op.x)])
        elif kind == "sigmoid_hard":
            emit("HardSigmoid", [tensor(op.x)], alpha=float(op.alpha.val), beta=float(op.beta.val))
        elif kind == "conv":
            assert op.pad_type.val in ("custom", "valid")
            top, bottom, left, right = map(int, op.pad.val)
            emit("Conv", [tensor(op.x), tensor(op.weight), tensor(op.bias)],
                 strides=list(map(int, op.strides.val)), dilations=list(map(int, op.dilations.val)),
                 group=int(op.groups.val), pads=[top,left,bottom,right])
        elif kind == "reduce_mean":
            emit("ReduceMean", [tensor(op.x)], axes=list(map(int, op.axes.val)), keepdims=int(op.keep_dims.val))
        elif kind == "upsample_bilinear":
            scales = [1,1,float(op.scale_factor_height.val),float(op.scale_factor_width.val)]
            emit("Resize", [tensor(op.x), constant("depthmini_empty_roi", []), constant(out+"_scales", scales)],
                 mode="linear", coordinate_transformation_mode="align_corners" if op.align_corners.val else "half_pixel")
        elif kind == "concat":
            assert not op.interleave.val
            emit("Concat", [tensor(x) for x in op.values], axis=int(op.axis.val))
        else:
            raise ValueError(f"Unsupported operation: {kind}")

    def io(name, channels):
        return helper.make_tensor_value_info(name, TensorProto.FLOAT, [1,channels,"height","width"])
    graph = helper.make_graph(nodes, "DepthMini", [io("image",3)], [io("depth",1)], list(constants.values()))
    model = helper.make_model(graph, producer_name="Compositor for Windows", opset_imports=[helper.make_opsetid("",11)])
    model.ir_version = 7
    onnx.checker.check_model(model)
    onnx.save_model(model, str(output))


if __name__ == "__main__":
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument("package", type=Path)
    args.add_argument("output", type=Path)
    opts = args.parse_args()
    convert(opts.package, opts.output)
