//! Tiny ONNX fixture for tract load tests (zeros policy + value).
//!
//! Not a trained net. HUD must not ship this (slice A plumbing only).

use dab_core::{AZ_CHANNELS, AZ_FEATURES, AZ_PLANE, AZ_POLICY};
use prost::Message;
use tract_onnx::pb::{
    attribute_proto, tensor_proto, tensor_shape_proto, type_proto, AttributeProto, GraphProto,
    ModelProto, NodeProto, OperatorSetIdProto, TensorProto, TensorShapeProto, TypeProto,
    ValueInfoProto, Version,
};

fn dim(v: i64) -> tensor_shape_proto::Dimension {
    tensor_shape_proto::Dimension {
        denotation: String::new(),
        value: Some(tensor_shape_proto::dimension::Value::DimValue(v)),
    }
}

fn tensor_info(name: &str, dims: &[i64]) -> ValueInfoProto {
    ValueInfoProto {
        name: name.into(),
        r#type: Some(TypeProto {
            denotation: String::new(),
            value: Some(type_proto::Value::TensorType(type_proto::Tensor {
                elem_type: tensor_proto::DataType::Float as i32,
                shape: Some(TensorShapeProto {
                    dim: dims.iter().copied().map(dim).collect(),
                }),
            })),
        }),
        doc_string: String::new(),
    }
}

fn zeros(name: &str, dims: &[i64]) -> TensorProto {
    let n: usize = dims.iter().map(|d| *d as usize).product();
    TensorProto {
        dims: dims.to_vec(),
        data_type: tensor_proto::DataType::Float as i32,
        name: name.into(),
        float_data: vec![0.0; n],
        ..Default::default()
    }
}

fn node(
    name: &str,
    op: &str,
    inputs: &[&str],
    outputs: &[&str],
    attribute: Vec<AttributeProto>,
) -> NodeProto {
    NodeProto {
        name: name.into(),
        op_type: op.into(),
        input: inputs.iter().map(|s| (*s).to_string()).collect(),
        output: outputs.iter().map(|s| (*s).to_string()).collect(),
        attribute,
        domain: String::new(),
        doc_string: String::new(),
    }
}

/// ONNX: Flatten 7×11×11 → MatMul zeros → policy 60 and value 1.
pub fn dummy_onnx() -> Vec<u8> {
    let w_pol = AZ_FEATURES as i64;
    let w_val = AZ_FEATURES as i64;
    let model = ModelProto {
        ir_version: Version::IrVersion as i64,
        producer_name: "dab-wasm-az-fixture".into(),
        opset_import: vec![OperatorSetIdProto {
            domain: String::new(),
            version: 13,
        }],
        graph: Some(GraphProto {
            name: "az-dummy".into(),
            input: vec![tensor_info(
                "features",
                &[1, AZ_CHANNELS as i64, AZ_PLANE as i64, AZ_PLANE as i64],
            )],
            output: vec![
                tensor_info("policy", &[1, AZ_POLICY as i64]),
                tensor_info("value", &[1, 1]),
            ],
            initializer: vec![
                zeros("W_pol", &[w_pol, AZ_POLICY as i64]),
                zeros("W_val", &[w_val, 1]),
            ],
            node: vec![
                node(
                    "flatten",
                    "Flatten",
                    &["features"],
                    &["flat"],
                    vec![AttributeProto {
                        name: "axis".into(),
                        r#type: attribute_proto::AttributeType::Int as i32,
                        i: 1,
                        ..Default::default()
                    }],
                ),
                node("pol", "MatMul", &["flat", "W_pol"], &["policy"], vec![]),
                node("val", "MatMul", &["flat", "W_val"], &["value"], vec![]),
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut buf = Vec::new();
    model.encode(&mut buf).expect("encode dummy onnx");
    buf
}
