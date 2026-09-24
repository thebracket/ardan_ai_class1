# Governance in the Real World

The first thing to say is the thing nobody wants to hear: **there is no single "governance
server".** Governance is a stack of layers, and different products solve different layers. When
somebody asks "what should we use for governance?", the useful answer is "for *which* layer?".

Here are the layers, what each answers, and the names you'll meet in each.

| Layer | The question it answers | Open source | Enterprise / managed |
| --- | --- | --- | --- |
| Identity | Who is calling? | SPIFFE/SPIRE, Keycloak, Ory, Zitadel | Okta/Auth0, Microsoft Entra Agent ID, Google Cloud Agent Identity, Ping, HashiCorp Vault |
| Policy (PDP) | Is this call allowed? | Open Policy Agent, Cedar, OpenFGA, SpiceDB, Cerbos, Casbin, Oso | AWS Verified Permissions, Permit.io, Aserto, Authzed Cloud, PlainID, Axiomatics |
| Guardrails | Is this content or action safe? | NeMo Guardrails, Guardrails AI, LLM Guard, Garak | Lakera Guard (Check Point), AWS Bedrock Guardrails, Azure AI Content Safety, Palo Alto Prisma AIRS, Cisco AI Defense |
| MCP / agent gateways | How do we discover, approve and reach all our tools? | Docker MCP Gateway, IBM ContextForge, Microsoft MCP Gateway, Lunar.dev MCPX, Obot | Docker MCP Enterprise Gateway, TrueFoundry, Composio, Lasso Security, MintMCP |
| GRC / AI governance | Can we prove we followed policy? | — | Credo AI, IBM watsonx.governance, Microsoft Purview, OneTrust, Arthur, Fiddler, Holistic AI, Zenity |
| Audit & observability | What actually happened? | OpenTelemetry, Langfuse, Arize Phoenix | LangSmith, Arize, Datadog LLM Observability, New Relic |

A few of these deserve a sentence more.

**Policy decision points** are the closest thing to the "governance server" of the previous page.
You send `(identity, action, resource)` and get back allow/deny. **OPA** (CNCF, Rego) and **Cedar**
(AWS, and the engine behind Amazon Verified Permissions) are the two general-purpose options;
**OpenFGA** and **SpiceDB** are for when permissions are *relationship-shaped* ("who can see this
document?") rather than a flat policy.

**MCP gateways and registries** are the layer most students actually mean. They front all your MCP
servers with one endpoint: a curated registry, authentication, rate limiting, secrets kept out of
process environments, and a single place to see every call. Docker's gateway runs servers in
containers and verifies signatures; IBM's ContextForge federates servers and APIs with RBAC and
OpenTelemetry. Once you have more than a handful of servers, this is the pain you feel first.

**The MCP authorization spec** is worth knowing because it's a *standard*, not a product: a
protected MCP server is an OAuth 2.1 resource server, discovering its authorization server via
Protected Resource Metadata (RFC 9728), with PKCE and resource indicators (RFC 8707). If your tools
are reachable over the network, this is the ground truth the products are built on.

## Choosing by size

* **One person, one harness.** Nothing. Identity is you, policy is a function, and the argument
  validation and provenance checks you've already written are your guardrails.
* **A team, several tools.** Put a policy decision point behind the tool dispatch, and start
  curating servers through a gateway/registry before the list gets long.
* **A regulated enterprise.** Add guardrails, then agent identity, then a GRC platform. In that
  order - a GRC tool with no runtime enforcement is paperwork.

## What won't churn

Vendor lists age badly, and this space moves weekly. The things worth teaching as durable are the
**standards**: OAuth 2.1 and the MCP authorization spec for identity, OPA/Cedar as the shape of a
policy decision, and OpenTelemetry for audit. Everything else is an implementation of one of those.

> The rule of thumb, everywhere: **add a layer when you feel the pain it removes.** A small harness
> with an identity, a policy function, and a log already has all the governance it needs - and it
> has it in about thirty lines.
