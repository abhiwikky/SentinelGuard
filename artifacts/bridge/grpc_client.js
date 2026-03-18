/**
 * SentinelGuard gRPC Client Factory
 *
 * Loads protobuf definitions and creates a gRPC client for
 * communicating with the Rust agent's SentinelGuardService.
 */

const grpc = require("@grpc/grpc-js");
const protoLoader = require("@grpc/proto-loader");
const path = require("path");

const PROTO_PATH = path.resolve(__dirname, "..", "proto", "sentinelguard.proto");
const DEFAULT_TARGET = "127.0.0.1:50051";

let _packageDefinition = null;
let _protoDescriptor = null;

/**
 * Load the protobuf definition (cached after first load)
 */
function loadProto() {
  if (_protoDescriptor) return _protoDescriptor;

  _packageDefinition = protoLoader.loadSync(PROTO_PATH, {
    keepCase: false,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true,
  });

  _protoDescriptor = grpc.loadPackageDefinition(_packageDefinition);
  return _protoDescriptor;
}

/**
 * Create a gRPC client instance
 * @param {string} target - gRPC server address (default: 127.0.0.1:50051)
 * @returns {object} gRPC client
 */
function createClient(target = DEFAULT_TARGET) {
  const proto = loadProto();
  const ServiceClass = proto.sentinelguard.SentinelGuardService;

  const client = new ServiceClass(
    target,
    grpc.credentials.createInsecure()
  );

  return client;
}

/**
 * Promisify a gRPC unary call
 */
function callUnary(client, method, request = {}) {
  return new Promise((resolve, reject) => {
    client[method](request, (error, response) => {
      if (error) {
        reject(error);
      } else {
        resolve(response);
      }
    });
  });
}

module.exports = {
  createClient,
  callUnary,
  loadProto,
  DEFAULT_TARGET,
};
