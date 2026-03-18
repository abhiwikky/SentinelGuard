const grpc = require("@grpc/grpc-js");
const path = require("path");
const { createClient, callUnary } = require("./grpc_client");

async function main() {
    console.log("\n=======================================================");
    console.log("  SentinelGuard Direct gRPC Verifier");
    console.log("=======================================================\n");
    console.log("Target: 127.0.0.1:50051 (Bypassing Node.js Bridge API)\n");

    try {
        const client = createClient("127.0.0.1:50051");

        console.log("\x1b[36m1. Checking Agent Health (getHealth)...\x1b[0m");
        const health = await callUnary(client, "getHealth", {});
        
        console.log(`  Agent Version      : ${health.agent_version}`);
        console.log(`  Driver Connected   : ${health.driver_connected ? '\x1b[32mTrue\x1b[0m' : '\x1b[31mFalse\x1b[0m'}`);
        console.log(`  Model Loaded       : ${health.model_loaded ? '\x1b[32mTrue\x1b[0m' : '\x1b[31mFalse\x1b[0m'}`);
        console.log(`  Database Connected : ${health.database_connected ? '\x1b[32mTrue\x1b[0m' : '\x1b[31mFalse\x1b[0m'}`);
        console.log("");
        console.log(`  Telemetry:`);
        console.log(`    Events Processed : ${health.events_processed}`);
        console.log(`    Events/sec       : ${health.events_per_second}`);
        console.log(`    Alerts Generated : ${health.alerts_generated}\n`);

        console.log("\x1b[36m2. Checking Risk Overview (getProcessRiskOverview)...\x1b[0m");
        const risk = await callUnary(client, "getProcessRiskOverview", { limit: 5 });
        if (!risk.processes || risk.processes.length === 0) {
            console.log("  \x1b[32mNo risky processes detected.\x1b[0m\n");
        } else {
            for (const p of risk.processes) {
                console.log(`  [PID: ${p.process_id}] ${p.process_name} - Score: ${p.risk_score.toFixed(2)}`);
            }
            console.log("");
        }

        console.log("\x1b[36m3. Fetching Recent Alerts (getAlerts)...\x1b[0m");
        const alertsReq = await callUnary(client, "getAlerts", { limit: 5 });
        if (!alertsReq.alerts || alertsReq.alerts.length === 0) {
            console.log("  \x1b[32mNo runtime alerts.\x1b[0m\n");
        } else {
            for (const a of alertsReq.alerts) {
                console.log(`  [PID: ${a.process_id}] ${a.process_name} - Severity: ${a.severity}`);
                console.log(`  ${a.description}`);
                console.log('  ---');
            }
            console.log("");
        }

        console.log("=======================================================");
        console.log("  \x1b[36mVerification Complete\x1b[0m");
        console.log("=======================================================\n");

        client.close();
        process.exit(0);

    } catch (err) {
        console.error("\x1b[31m  CRITICAL ERROR: Failed to communicate with Rust Agent gRPC.\x1b[0m");
        console.error("  Is the agent (sentinelguard_agent.exe) running?");
        console.error(`  Details: ${err.message}\n`);
        process.exit(1);
    }
}

main();
