#!/usr/bin/env python3
"""
Script de normalização e resolução de bornes da biblioteca canônica (P3-#17).
Implementa normas IEC 60947-1, IEC 60204-1, EN 50005, IEC 81346.
Garante que 100% dos elementos cabáveis possuam nomes de terminais não-vazios e únicos.
Atualiza tanto catalog.json quanto os atributos data-name nos arquivos SVG em disco.
"""

import json
import os
import re
import sys
from collections import Counter

def is_nc(name):
    n = name.lower()
    return any(k in n for k in ["nc", "nf", "fechado", "ferme", "geschlossen", "break", "emergency", "parada", "desarme"])

def is_no(name):
    n = name.lower()
    return any(k in n for k in ["no", "na", "aberto", "ouvert", "schliesser", "make", "partida"])

def normalize_element_terminals(el):
    kind = el.get("kind")
    if kind == "block":
        return el.get("terminals", [])
    
    terms = el.get("terminals", [])
    if not terms:
        return []
    
    n_terms = len(terms)
    names = [t.get("name", "").strip() for t in terms]
    
    # Se já tem nomes únicos e não-vazios, preserva
    if all(n != "" for n in names) and len(names) == len(set(names)):
        return terms
    
    name_pt = el.get("name_pt") or ""
    name_en = el.get("name_en") or ""
    slug = el.get("slug") or ""
    combined = f"{slug} {name_pt} {name_en}".lower()
    
    assigned = [None] * n_terms
    
    # Índices ordenados por (y, x) - top-to-bottom, left-to-right
    by_y_x = sorted(range(n_terms), key=lambda i: (terms[i]["y"], terms[i]["x"]))
    # Índices ordenados por (x, y) - left-to-right, top-to-bottom
    by_x_y = sorted(range(n_terms), key=lambda i: (terms[i]["x"], terms[i]["y"]))
    
    # 1. Bobinas
    if kind == "relay_coil":
        if n_terms == 2:
            assigned[by_y_x[0]] = "A1"
            assigned[by_y_x[1]] = "A2"
        elif n_terms == 3:
            assigned[by_y_x[0]] = "A1"
            assigned[by_y_x[1]] = "A2"
            assigned[by_y_x[2]] = "A3"
        elif n_terms == 4:
            assigned[by_x_y[0]] = "A1"
            assigned[by_x_y[1]] = "A2"
            assigned[by_x_y[2]] = "B1"
            assigned[by_x_y[3]] = "B2"
            
    # 2. Sinalizadores / Lâmpadas
    elif kind == "lamp":
        if n_terms == 2:
            assigned[by_y_x[0]] = "X1"
            assigned[by_y_x[1]] = "X2"
        else:
            for r, idx in enumerate(by_x_y):
                assigned[idx] = f"X{r+1}"
                
    # 3. Contatos térmicos
    elif kind == "thermal_contact":
        if n_terms == 2:
            if is_no(combined) and not is_nc(combined):
                assigned[by_y_x[0]] = "97"
                assigned[by_y_x[1]] = "98"
            else:
                assigned[by_y_x[0]] = "95"
                assigned[by_y_x[1]] = "96"
        elif n_terms == 3:
            assigned[by_y_x[0]] = "95"
            assigned[by_y_x[1]] = "96"
            assigned[by_y_x[2]] = "98"
        elif n_terms == 4:
            assigned[by_x_y[0]] = "95"
            assigned[by_x_y[1]] = "96"
            assigned[by_x_y[2]] = "97"
            assigned[by_x_y[3]] = "98"
            
    # 4. Motores
    elif kind == "motor":
        if n_terms == 2:
            assigned[by_y_x[0]] = "U1"
            assigned[by_y_x[1]] = "U2"
        elif n_terms == 3:
            assigned[by_x_y[0]] = "U"
            assigned[by_x_y[1]] = "V"
            assigned[by_x_y[2]] = "W"
        elif n_terms == 4:
            assigned[by_x_y[0]] = "U"
            assigned[by_x_y[1]] = "V"
            assigned[by_x_y[2]] = "W"
            assigned[by_x_y[3]] = "PE"
        elif n_terms == 6:
            top = sorted(by_y_x[:3], key=lambda i: terms[i]["x"])
            bot = sorted(by_y_x[3:], key=lambda i: terms[i]["x"])
            assigned[top[0]] = "U1"
            assigned[top[1]] = "V1"
            assigned[top[2]] = "W1"
            assigned[bot[0]] = "U2"
            assigned[bot[1]] = "V2"
            assigned[bot[2]] = "W2"
            
    # 5. Botoeiras e Chaves
    elif kind in ["momentary_button", "latching_switch", "limit_switch", "emergency_stop", "selector_switch"]:
        if n_terms == 2:
            if kind == "emergency_stop" or (is_nc(combined) and not is_no(combined)):
                assigned[by_y_x[0]] = "11"
                assigned[by_y_x[1]] = "12"
            else:
                assigned[by_y_x[0]] = "13"
                assigned[by_y_x[1]] = "14"
        elif n_terms == 3:
            assigned[by_y_x[0]] = "11"
            assigned[by_y_x[1]] = "12"
            assigned[by_y_x[2]] = "14"
        elif n_terms == 4:
            assigned[by_x_y[0]] = "13"
            assigned[by_x_y[1]] = "14"
            assigned[by_x_y[2]] = "21"
            assigned[by_x_y[3]] = "22"
            
    # 6. Aparelhos de Proteção e Contatos de Potência
    elif kind in ["circuit_breaker", "fuse", "disconnector", "contactor_contact"]:
        if n_terms % 2 == 0:
            num_poles = n_terms // 2
            ys = [t["y"] for t in terms]
            sorted_ys = sorted(ys)
            median_y = sorted_ys[num_poles]
            
            top_half = [i for i in range(n_terms) if terms[i]["y"] < median_y]
            bot_half = [i for i in range(n_terms) if terms[i]["y"] >= median_y]
            
            if len(top_half) == num_poles and len(bot_half) == num_poles:
                top_sorted = sorted(top_half, key=lambda i: terms[i]["x"])
                bot_sorted = sorted(bot_half, key=lambda i: terms[i]["x"])
                for p in range(num_poles):
                    assigned[top_sorted[p]] = str(2 * p + 1)
                    assigned[bot_sorted[p]] = str(2 * p + 2)
            else:
                # Disposição horizontal (partição por x)
                xs = [t["x"] for t in terms]
                sorted_xs = sorted(xs)
                median_x = sorted_xs[num_poles]
                left_half = [i for i in range(n_terms) if terms[i]["x"] < median_x]
                right_half = [i for i in range(n_terms) if terms[i]["x"] >= median_x]
                if len(left_half) == num_poles and len(right_half) == num_poles:
                    left_sorted = sorted(left_half, key=lambda i: terms[i]["y"])
                    right_sorted = sorted(right_half, key=lambda i: terms[i]["y"])
                    for p in range(num_poles):
                        assigned[left_sorted[p]] = str(2 * p + 1)
                        assigned[right_sorted[p]] = str(2 * p + 2)
                        
    # 7. Fontes
    elif kind == "source":
        if n_terms == 1:
            if any(k in combined for k in ["pe", "terra", "earth"]):
                assigned[0] = "PE"
            elif any(k in combined for k in ["neutro", "neutral"]):
                assigned[0] = "N"
            elif "-" in combined or "negativo" in combined:
                assigned[0] = "-"
            elif "+" in combined or "positivo" in combined:
                assigned[0] = "+"
            else:
                assigned[0] = "L"
        elif n_terms == 2:
            if any(k in combined for k in ["+", "-", "cc", "dc"]):
                assigned[by_x_y[0]] = "+"
                assigned[by_x_y[1]] = "-"
            else:
                assigned[by_x_y[0]] = "L"
                assigned[by_x_y[1]] = "N"
        elif n_terms == 3:
            assigned[by_x_y[0]] = "L1"
            assigned[by_x_y[1]] = "L2"
            assigned[by_x_y[2]] = "L3"
        elif n_terms == 4:
            assigned[by_x_y[0]] = "L1"
            assigned[by_x_y[1]] = "L2"
            assigned[by_x_y[2]] = "L3"
            if any(k in combined for k in ["pe", "terra", "earth"]):
                assigned[by_x_y[3]] = "PE"
            else:
                assigned[by_x_y[3]] = "N"
        elif n_terms == 5:
            assigned[by_x_y[0]] = "L1"
            assigned[by_x_y[1]] = "L2"
            assigned[by_x_y[2]] = "L3"
            assigned[by_x_y[3]] = "N"
            assigned[by_x_y[4]] = "PE"
            
    # 8. Terminais de passagem
    elif kind == "terminal":
        if n_terms == 2:
            assigned[by_y_x[0]] = "1"
            assigned[by_y_x[1]] = "2"
        else:
            for r, idx in enumerate(by_y_x):
                assigned[idx] = str(r + 1)

    # 9. Preserva e desambigua nomes existentes válidos (não-vazios)
    used_names = set(a for a in assigned if a is not None)
    seen_existing = {}
    for idx in range(n_terms):
        curr = terms[idx].get("name", "").strip()
        if assigned[idx] is None and curr != "":
            count = seen_existing.get(curr, 0) + 1
            seen_existing[curr] = count
            if count == 1 and curr not in used_names:
                cand = curr
            else:
                suffix = count
                cand = f"{curr}_{suffix}"
                while cand in used_names:
                    suffix += 1
                    cand = f"{curr}_{suffix}"
            used_names.add(cand)
            assigned[idx] = cand
            
    # 10. Preenchimento de qualquer borne restante com numeração sequencial 1, 2, 3...
    counter = 1
    for idx in by_y_x:
        if assigned[idx] is None:
            while str(counter) in used_names:
                counter += 1
            cand = str(counter)
            assigned[idx] = cand
            used_names.add(cand)
            counter += 1
            
    # 11. Desduplicação estrita (evita colisões mesmo em componentes complexos)
    seen = {}
    for idx in range(n_terms):
        val = assigned[idx] or str(idx + 1)
        if val in seen:
            seen[val] += 1
            assigned[idx] = f"{val}_{seen[val]}"
        else:
            seen[val] = 1
            assigned[idx] = val
            
    # Atualiza a lista de terminais do elemento
    new_terms = []
    for idx, t in enumerate(terms):
        new_term = dict(t)
        new_term["name"] = assigned[idx]
        new_terms.append(new_term)
        
    return new_terms

def update_svg_file(svg_path, terminals):
    if not os.path.exists(svg_path) or not terminals:
        return 0
    with open(svg_path, "r", encoding="utf-8", errors="ignore") as f:
        content = f.read()
        
    if "qet-terminal" not in content:
        return 0
        
    # Regex para localizar <circle ... class="qet-terminal" ... />
    circle_re = re.compile(r'(<circle\s+[^>]*class="qet-terminal"[^>]*>)', re.IGNORECASE)
    
    # Parse attributes of each circle
    def replacer(match):
        tag = match.group(1)
        cx_m = re.search(r'cx="([0-9\.\-]+)"', tag)
        cy_m = re.search(r'cy="([0-9\.\-]+)"', tag)
        if not cx_m or not cy_m:
            return tag
        cx = float(cx_m.group(1))
        cy = float(cy_m.group(1))
        
        # Encontra o terminal mais próximo por (x, y)
        best_term = None
        best_dist = 1e9
        for t in terminals:
            dist = (t["x"] - cx)**2 + (t["y"] - cy)**2
            if dist < best_dist:
                best_dist = dist
                best_term = t
                
        if best_term and best_dist < 4.0: # dentro de 2px de tolerância
            new_name = best_term["name"]
            # Substitui data-name="..." ou data-label="..."
            if 'data-name="' in tag:
                tag = re.sub(r'data-name="[^"]*"', f'data-name="{new_name}"', tag)
            elif 'data-label="' in tag:
                tag = re.sub(r'data-label="[^"]*"', f'data-label="{new_name}"', tag)
            else:
                tag = tag[:-1] + f' data-name="{new_name}">'
        return tag
        
    new_content = circle_re.sub(replacer, content)
    if new_content != content:
        with open(svg_path, "w", encoding="utf-8") as f:
            f.write(new_content)
        return 1
    return 0

def main():
    base_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    catalog_path = os.path.join(base_dir, "symbols_qet", "catalog.json")
    if not os.path.exists(catalog_path):
        print(f"Erro: catálogo não encontrado em {catalog_path}")
        sys.exit(1)
        
    print(f"Carregando catálogo de {catalog_path}...")
    with open(catalog_path, "r", encoding="utf-8") as f:
        catalog = json.load(f)
        
    elements = catalog.get("elements", [])
    print(f"Total de elementos carregados: {len(elements)}")
    
    normalized_count = 0
    total_wireable = 0
    svgs_updated = 0
    
    for el in elements:
        if el.get("kind") == "block":
            continue
        total_wireable += 1
        orig_names = [t.get("name", "") for t in el.get("terminals", [])]
        norm_terms = normalize_element_terminals(el)
        el["terminals"] = norm_terms
        new_names = [t.get("name", "") for t in norm_terms]
        
        if orig_names != new_names:
            normalized_count += 1
            
        # Atualiza arquivos SVG correspondentes
        fam = el.get("family", "")
        svg_base = el.get("svg_base", "")
        if svg_base:
            svg_path = os.path.join(base_dir, "symbols_qet", fam, svg_base)
            svgs_updated += update_svg_file(svg_path, norm_terms)
            
        for state in el.get("states", []):
            st_svg = state.get("svg")
            if st_svg:
                st_path = os.path.join(base_dir, "symbols_qet", fam, st_svg)
                svgs_updated += update_svg_file(st_path, norm_terms)
                
    # Validação rigorosa pós-normalização
    empty_errors = 0
    dup_errors = 0
    for el in elements:
        if el.get("kind") == "block":
            continue
        names = [t.get("name", "").strip() for t in el.get("terminals", [])]
        if any(n == "" for n in names):
            empty_errors += 1
            print(f"[ERRO] Elemento {el.get('id')} ainda tem bornes vazios: {names}")
        if len(names) != len(set(names)):
            dup_errors += 1
            print(f"[ERRO] Elemento {el.get('id')} tem bornes duplicados: {names}")
            
    print("==================================================")
    print(f"Total de elementos cabáveis: {total_wireable}")
    print(f"Elementos com bornes normalizados: {normalized_count}")
    print(f"Arquivos SVG atualizados em disco: {svgs_updated}")
    print(f"Erros de bornes vazios: {empty_errors}")
    print(f"Erros de bornes duplicados: {dup_errors}")
    
    if empty_errors == 0 and dup_errors == 0:
        print("[SUCESSO] 100% dos elementos cabáveis validados com bornes únicos e não-vazios!")
        # Salva o catálogo atualizado
        with open(catalog_path, "w", encoding="utf-8") as f:
            json.dump(catalog, f, indent=2, ensure_ascii=False)
        print(f"Catálogo salvo com sucesso em {catalog_path}")
    else:
        print("[FALHA] Validação encontrou inconsistências.")
        sys.exit(1)

if __name__ == "__main__":
    main()
