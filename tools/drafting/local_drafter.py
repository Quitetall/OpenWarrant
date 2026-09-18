#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Local llama.cpp drafting adapter. Emits unreviewed proposals, never applies them."""
import argparse
import ipaddress
import json
import sys
import urllib.parse
import urllib.request

LIMIT = 262144
ATOMS = (("intent", 10, "10-intent.md"), ("basis", 20, "20-basis.md"),
         ("work_order", 40, "40-work-order.md"), ("milestones", 45, "45-milestones.yaml"),
         ("assurance", 60, "60-assurance.md"))


MILESTONES = """schema: oh.war/milestones/v1
milestones:
  - id: M1
    title: Deliver requested outcome
    stage_refs: [STAGE-001]
    obligation_refs: [OBL-001]
stages:
  - id: STAGE-001
    title: Implement and check requested outcome
    executor_kind: agent
    responsibility_tier: T1
"""


def object_schema(properties):
    return {"type": "object", "properties": properties, "required": list(properties), "additionalProperties": False}


def text_schema(limit):
    return {"type": "string", "minLength": 1, "maxLength": limit}


def proposal_schema():
    atoms = [object_schema({"op": {"const": "create_atom"}, "role": {"const": role},
                            "ordinal": {"const": ordinal}, "path": {"const": path},
                            "body": {"const": MILESTONES} if role == "milestones" else text_schema(1600)}) for role, ordinal, path in ATOMS]
    # llama.cpp supports tuple items; this is backend guidance, not the OW schema.
    return object_schema({"api_version": {"const": "oh.war/draft-proposal/v2"},
                          "proposed_identity": object_schema({"title": text_schema(160), "profile": {"const": "delivery"}, "assurance": {"const": "basic"}}),
                          "operations": {"type": "array", "items": atoms, "minItems": 5, "maxItems": 5, "additionalItems": False},
                          "risk_assessment": text_schema(1200)})


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def loads(raw):
    return json.loads(raw, object_pairs_hook=unique_object,
                      parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON value")))


def check_schema(value, schema):
    if "const" in schema:
        if type(value) is not type(schema["const"]) or value != schema["const"]:
            raise ValueError("response differs from required atom field")
        return
    kind = schema["type"]
    if kind == "object":
        if not isinstance(value, dict) or set(value) != set(schema["properties"]):
            raise ValueError("response has missing or unexpected fields")
        for key, child in schema["properties"].items():
            check_schema(value[key], child)
    elif kind == "array":
        if not isinstance(value, list) or len(value) != len(schema["items"]):
            raise ValueError("response must contain exactly five atoms")
        for item, child in zip(value, schema["items"]):
            check_schema(item, child)
    elif kind == "string":
        if not isinstance(value, str) or not value.strip() or not schema["minLength"] <= len(value) <= schema["maxLength"]:
            raise ValueError("response text is empty or over limit")
    else:
        raise ValueError("unsupported adapter constraint")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ValueError("backend redirect refused")


def endpoint_url(endpoint):
    parsed = urllib.parse.urlsplit(endpoint)
    try:
        local = ipaddress.ip_address(parsed.hostname or "").is_loopback
    except ValueError:
        local = False
    if parsed.scheme != "http" or not local or parsed.username or parsed.password or parsed.query or parsed.fragment or parsed.path not in ("", "/"):
        raise ValueError("backend must be an explicit loopback HTTP origin without credentials")
    return endpoint.rstrip("/") + "/v1/chat/completions"


def draft(request, endpoint, model):
    if not isinstance(request, dict) or request.get("api_version") != "oh.war/draft-request/v1" or not isinstance(request.get("user_request"), str) or not request["user_request"].strip():
        raise ValueError("expected canonical draft request with user_request")
    if request.get("profile", "delivery") != "delivery" or request.get("assurance", "basic") != "basic":
        raise ValueError("local adapter supports only delivery/basic requests")
    schema = proposal_schema()
    system = ("Draft only the requested bounded delivery. No tools, execution, approval or invented evidence. "
              "Return JSON matching the supplied schema. The schema fixes each role, ordinal and filename. "
              "The final user message is the exact task. The preceding JSON is request context; "
              "preserve its constraints and uncertainties. Existing Warrant inventory is background, not requested scope. "
              "Do not select an existing Warrant or invent an identifier. Use a plain descriptive title. "
              "Do not enumerate inventories. Keep bodies concise. "
              "Intent states outcome and scope. Basis names uncertainty, never inventing source contents. "
              "Work order states deliverables, constraints and rollback. Milestones body is the exact fixed template in the schema. "
              "Assurance includes heading '### OBL-001 — outcome', "
              "'- **scope:**' and '- **evidence:**' with concrete positive and refusal checks. "
              "No source frontmatter. No durable architecture decisions: expose uncertainty in risk_assessment for human review.")
    context = {key: value for key, value in request.items() if key != "user_request"}
    body = {"model": model, "messages": [{"role": "system", "content": system},
            {"role": "user", "content": json.dumps(context)},
            {"role": "user", "content": request["user_request"]}],
            "temperature": 0, "max_tokens": 1600, "response_format": {"type": "json_object", "schema": schema},
            "chat_template_kwargs": {"enable_thinking": False}}
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    req = urllib.request.Request(endpoint_url(endpoint), data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
    with opener.open(req, timeout=240) as response:
        raw = response.read(LIMIT + 1)
    if len(raw) > LIMIT:
        raise ValueError("backend response exceeds limit")
    result = loads(raw)
    choices = result.get("choices") if isinstance(result, dict) else None
    if not isinstance(choices, list) or len(choices) != 1 or not isinstance(choices[0], dict) or choices[0].get("finish_reason") != "stop":
        raise ValueError("backend did not return one complete response")
    message = choices[0].get("message")
    if not isinstance(message, dict) or not isinstance(message.get("content"), str):
        raise ValueError("backend returned no proposal text")
    proposal = loads(message["content"])
    check_schema(proposal, schema)
    return proposal


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--endpoint", required=True)
    parser.add_argument("--model", required=True)
    args = parser.parse_args()
    try:
        raw = sys.stdin.buffer.read(65537)
        if len(raw) > 65536:
            raise ValueError("draft request exceeds 64 KiB")
        proposal = draft(loads(raw), args.endpoint, args.model)
        print(json.dumps(proposal, ensure_ascii=False, allow_nan=False))
        return 0
    except (OSError, ValueError, TypeError, KeyError) as error:
        print(f"local drafter refused: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
