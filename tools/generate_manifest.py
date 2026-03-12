#!/usr/bin/env python3
from __future__ import annotations

import ast
import json
import pathlib
import re
import subprocess
import sys
from collections import OrderedDict
from datetime import datetime, timezone


CONFIG_DIR = pathlib.Path("MCP") / "Kis Trading MCP" / "configs"
EXAMPLES_ROOT = pathlib.Path("examples_llm")
INTERNAL_PARAMS = {
    "env_dv",
    "tr_cont",
    "dataframe",
    "dataframe1",
    "dataframe2",
    "depth",
    "max_depth",
    "fk100",
    "nk100",
    "fk200",
    "nk200",
}
SPECIAL_TR_ID_RESOLVERS = {
    ("domestic-stock", "order_cash"): "domestic_stock.order_cash",
    ("domestic-stock", "inquire_daily_ccld"): "domestic_stock.inquire_daily_ccld",
    ("domestic-futureoption", "order"): "domestic_futureoption.order",
    ("domestic-futureoption", "order_rvsecncl"): "domestic_futureoption.order_rvsecncl",
    ("overseas-stock", "order"): "overseas_stock.order",
    ("overseas-stock", "order_resv"): "overseas_stock.order_resv",
}
AUTO_PARAM_SOURCES = {
    "cano": "account_no",
    "acnt_prdt_cd": "account_product_code",
    "appkey": "app_key",
    "appsecret": "app_secret",
    "secretkey": "app_secret",
    "htsid": "hts_id",
    "my_htsid": "hts_id",
}


def main() -> int:
    if len(sys.argv) != 3:
        print(
            "usage: generate_manifest.py <open-trading-api-root> <output-json>",
            file=sys.stderr,
        )
        return 1

    source_root = pathlib.Path(sys.argv[1]).expanduser().resolve()
    output_path = pathlib.Path(sys.argv[2]).expanduser().resolve()

    manifest = build_manifest(source_root)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {output_path}")
    return 0


def build_manifest(source_root: pathlib.Path) -> dict:
    config_root = source_root / CONFIG_DIR
    categories: list[dict] = []
    apis: list[dict] = []

    for config_path in sorted(config_root.glob("*.json")):
        config = json.loads(config_path.read_text(encoding="utf-8"))
        category_id = config_path.stem.replace("_", "-")
        categories.append(
            {
                "id": category_id,
                "config_file": config_path.name,
                "introduce": config.get("tool_info", {}).get("introduce", ""),
                "introduce_append": config.get("tool_info", {}).get("introduce_append", ""),
                "api_count": len(config.get("apis", {})),
            }
        )

        for api_type, api_info in config.get("apis", {}).items():
            entry = build_api_entry(source_root, category_id, api_type, api_info)
            apis.append(entry)

    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "source_repo": "https://github.com/koreainvestment/open-trading-api",
        "source_commit": git_commit(source_root),
        "category_count": len(categories),
        "api_count": len(apis),
        "categories": categories,
        "apis": apis,
    }


def build_api_entry(
    source_root: pathlib.Path,
    category_id: str,
    api_type: str,
    api_info: dict,
) -> dict:
    relative_folder = api_info["github_url"].split("/tree/main/")[-1]
    source_dir = source_root / relative_folder
    source_file = source_dir / f"{api_info['method']}.py"
    source_text = source_file.read_text(encoding="utf-8")

    request_fields = extract_request_fields(source_text, api_info["method"])
    pagination = extract_pagination(request_fields)
    tr_id_info = extract_tr_id(source_text, category_id, api_type)
    method = extract_http_method(source_text)
    post_uses_hashkey = method == "POST" and api_info["api_path"] not in ("/oauth2/tokenP", "/oauth2/Approval")

    params = []
    for name, param_info in api_info.get("params", {}).items():
        auto_source = AUTO_PARAM_SOURCES.get(name)
        hidden = (
            name in INTERNAL_PARAMS
            or param_info.get("type") == "pd.DataFrame"
            or "시스템 자동 설정" in (param_info.get("description") or "")
        )
        params.append(
            {
                "name": name,
                "cli_name": name.replace("_", "-"),
                "type": param_info.get("type", "str"),
                "required": bool(param_info.get("required", False) and not auto_source and name != "env_dv"),
                "default_value": param_info.get("default_value"),
                "description": param_info.get("description", ""),
                "hidden": hidden or name == "env_dv",
                "auto_source": auto_source,
            }
        )

    return {
        "id": f"{category_id}.{api_type.replace('_', '-')}",
        "category_id": category_id,
        "api_type": api_type,
        "command_name": api_type.replace("_", "-"),
        "category_label": api_info.get("category", ""),
        "display_name": api_info.get("name", ""),
        "method_name": api_info.get("method", ""),
        "api_path": api_info.get("api_path", ""),
        "http_method": method,
        "github_url": api_info.get("github_url", ""),
        "source_file": str(source_file.relative_to(source_root)),
        "params": params,
        "request_fields": request_fields,
        "pagination": pagination,
        "tr_id": tr_id_info,
        "post_uses_hashkey": post_uses_hashkey,
    }


def git_commit(source_root: pathlib.Path) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(source_root), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        )
        return result.stdout.strip()
    except Exception:
        return ""


def extract_http_method(source_text: str) -> str:
    if "requests.post(" in source_text:
        return "POST"
    if "postFlag=True" in source_text:
        return "POST"
    return "GET"


def extract_tr_id(source_text: str, category_id: str, api_type: str) -> dict:
    special = SPECIAL_TR_ID_RESOLVERS.get((category_id, api_type))
    if special:
        return {"kind": "special", "resolver": special}

    if "auth_token" in api_type:
        return {"kind": "none"}
    if "auth_ws_token" in api_type:
        return {"kind": "none"}

    env_match = re.search(
        r'if\s+env_dv\s*==\s*"real"\s*:\s*\n(?:\s*#.*\n)*\s*tr_id\s*=\s*"([^"]+)"\s*'
        r'\n\s*elif\s+env_dv\s*==\s*"demo"\s*:\s*\n(?:\s*#.*\n)*\s*tr_id\s*=\s*"([^"]+)"',
        source_text,
    )
    if env_match:
        return {
            "kind": "env",
            "real": env_match.group(1),
            "demo": env_match.group(2),
        }

    matches = re.findall(r'\n\s*tr_id\s*=\s*"([^"]+)"', source_text)
    unique_matches = []
    for match in matches:
        if match not in unique_matches:
            unique_matches.append(match)
    if len(unique_matches) == 1:
        return {"kind": "const", "value": unique_matches[0]}

    if not unique_matches:
        return {"kind": "none"}

    return {"kind": "unsupported", "candidates": unique_matches}


def extract_request_fields(source_text: str, method_name: str) -> list[dict]:
    module = ast.parse(source_text)
    function = None
    for node in module.body:
        if isinstance(node, ast.FunctionDef) and node.name == method_name:
            function = node
            break

    if function is None:
        return []

    fields: OrderedDict[str, dict] = OrderedDict()
    params_seen = False

    for node in ast.walk(function):
        if isinstance(node, ast.Assign):
            if any(isinstance(target, ast.Name) and target.id == "params" for target in node.targets):
                if isinstance(node.value, ast.Dict):
                    params_seen = True
                    for key_node, value_node in zip(node.value.keys, node.value.values):
                        key = ast_value_to_string(key_node)
                        if key is None:
                            continue
                        fields[key] = field_entry(value_node)

        if isinstance(node, ast.Assign):
            for target in node.targets:
                if (
                    isinstance(target, ast.Subscript)
                    and isinstance(target.value, ast.Name)
                    and target.value.id == "params"
                ):
                    key = ast_value_to_string(target.slice)
                    if key is None:
                        continue
                    fields[key] = field_entry(node.value)
                    params_seen = True

    if not params_seen:
        return []

    return [{"request_name": key, **value} for key, value in fields.items()]


def extract_pagination(request_fields: list[dict]) -> dict | None:
    fk_field = None
    nk_field = None

    for field in request_fields:
        name = field["request_name"]
        if name.startswith("CTX_AREA_FK"):
            fk_field = name
        elif name.startswith("CTX_AREA_NK"):
            nk_field = name

    if not fk_field and not nk_field:
        return None

    return {"ctx_fk_field": fk_field, "ctx_nk_field": nk_field}


def field_entry(node: ast.AST) -> dict:
    if isinstance(node, ast.Name):
        return {"source_param": node.id, "literal": None}
    if isinstance(node, ast.Constant):
        return {"source_param": None, "literal": node.value}
    return {"source_param": None, "literal": None}


def ast_value_to_string(node: ast.AST | None) -> str | None:
    if node is None:
        return None
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    if isinstance(node, ast.Index):
        return ast_value_to_string(node.value)
    return None


if __name__ == "__main__":
    raise SystemExit(main())
