#!/usr/bin/env python3
"""
Padroniza todos os SVGs da biblioteca (symbols_qet_all e symbols_qet)
conforme o padrão rígido de grade e bornes:
- Módulo do Grid: 48 x 48 px (4 x 4 mm a 300 DPI, distância entre bornes adjacentes).
- Meio Módulo (Sub-grid): 24 x 24 px (2 x 2 mm a 300 DPI).
- Bornes de Ligação: diâmetro de 8 px (raio de 4 px / 0.67 mm).
"""

import os
import glob
import re
import json
import xml.etree.ElementTree as ET

ET.register_namespace('', 'http://www.w3.org/2000/svg')

def determine_beta(root):
    terms = [el for el in root.iter() if el.tag.endswith('circle') and 'qet-terminal' in el.attrib.get('class', '')]
    if not terms:
        return 2.4  # Padrão base QET (20 px * 2.4 = 48 px)
    
    pts = []
    for t in terms:
        try:
            cx = float(t.attrib.get('cx', 0))
            cy = float(t.attrib.get('cy', 0))
            pts.append((cx, cy))
        except ValueError:
            pass
            
    unique_pts = []
    for p in pts:
        if not any(abs(u[0] - p[0]) < 0.5 and abs(u[1] - p[1]) < 0.5 for u in unique_pts):
            unique_pts.append(p)
            
    if len(unique_pts) < 2:
        return 2.4
        
    # 1. Passo horizontal entre bornes
    min_dx = float('inf')
    for i in range(len(unique_pts)):
        for j in range(i + 1, len(unique_pts)):
            if abs(unique_pts[i][1] - unique_pts[j][1]) <= 2.5:
                dx = abs(unique_pts[i][0] - unique_pts[j][0])
                if dx >= 5.0 and dx < min_dx:
                    min_dx = dx
                    
    if min_dx < float('inf'):
        if min_dx >= 15.0:
            return 48.0 / min_dx
        else:
            return 2.4
            
    # 2. Passo vertical entre bornes
    min_dy = float('inf')
    for i in range(len(unique_pts)):
        for j in range(i + 1, len(unique_pts)):
            if abs(unique_pts[i][0] - unique_pts[j][0]) <= 2.5:
                dy = abs(unique_pts[i][1] - unique_pts[j][1])
                if dy >= 8.0 and dy < min_dy:
                    min_dy = dy
                    
    if min_dy < float('inf'):
        if abs(min_dy - 20.0) <= 3.0:
            return 2.4
        elif abs(min_dy - 40.0) <= 3.0:
            return 1.2
        elif abs(min_dy - 60.0) <= 3.0:
            return 2.4
        elif abs(min_dy - 80.0) <= 3.0:
            return 1.2
        elif min_dy >= 15.0:
            return 48.0 / min_dy
            
    return 2.4

def scale_svg_file(path):
    try:
        tree = ET.parse(path)
    except Exception as e:
        print(f"Erro parseando {path}: {e}")
        return False
        
    root = tree.getroot()
    if root.attrib.get('data-standard') == '48px':
        return False
        
    beta = determine_beta(root)
    
    # Atualiza width e height
    if 'width' in root.attrib:
        try:
            w = float(root.attrib['width']) * beta
            root.attrib['width'] = f"{w:.1f}"
        except ValueError:
            pass
    if 'height' in root.attrib:
        try:
            h = float(root.attrib['height']) * beta
            root.attrib['height'] = f"{h:.1f}"
        except ValueError:
            pass
            
    # Atualiza viewBox
    if 'viewBox' in root.attrib:
        parts = root.attrib['viewBox'].split()
        if len(parts) == 4:
            try:
                vb = [float(p) * beta for p in parts]
                root.attrib['viewBox'] = f"{vb[0]:.1f} {vb[1]:.1f} {vb[2]:.1f} {vb[3]:.1f}"
            except ValueError:
                pass
                
    # Atualiza elementos filhos
    for el in root.iter():
        tag = el.tag.split('}')[-1]
        
        if 'stroke-width' in el.attrib:
            try:
                sw = float(el.attrib['stroke-width'])
                el.attrib['stroke-width'] = f"{min(sw * 1.5, 2.4):.1f}"
            except ValueError:
                pass
                
        if tag == 'line':
            for a in ('x1', 'y1', 'x2', 'y2'):
                if a in el.attrib:
                    try:
                        val = float(el.attrib[a]) * beta
                        el.attrib[a] = f"{val:.2f}"
                    except ValueError:
                        pass
                        
        elif tag == 'rect':
            for a in ('x', 'y', 'width', 'height', 'rx', 'ry'):
                if a in el.attrib:
                    try:
                        val = float(el.attrib[a]) * beta
                        el.attrib[a] = f"{val:.2f}"
                    except ValueError:
                        pass
                        
        elif tag == 'circle':
            is_term = 'qet-terminal' in el.attrib.get('class', '')
            for a in ('cx', 'cy'):
                if a in el.attrib:
                    try:
                        val = float(el.attrib[a]) * beta
                        el.attrib[a] = f"{val:.2f}"
                    except ValueError:
                        pass
            if is_term:
                el.attrib['r'] = '4.0'
            elif 'r' in el.attrib:
                try:
                    val = float(el.attrib['r']) * beta
                    el.attrib['r'] = f"{val:.2f}"
                except ValueError:
                    pass
                    
        elif tag == 'ellipse':
            for a in ('cx', 'cy', 'rx', 'ry'):
                if a in el.attrib:
                    try:
                        val = float(el.attrib[a]) * beta
                        el.attrib[a] = f"{val:.2f}"
                    except ValueError:
                        pass
                        
        elif tag in ('polyline', 'polygon'):
            if 'points' in el.attrib:
                pts = el.attrib['points'].strip()
                scaled_pts = []
                for pair in pts.split():
                    if ',' in pair:
                        try:
                            px, py = pair.split(',', 1)
                            scaled_pts.append(f"{float(px)*beta:.2f},{float(py)*beta:.2f}")
                        except ValueError:
                            scaled_pts.append(pair)
                el.attrib['points'] = ' '.join(scaled_pts)
                
        elif tag == 'path':
            if 'd' in el.attrib:
                def repl_arc(m):
                    m_x, m_y = float(m.group(1)) * beta, float(m.group(2)) * beta
                    rx, ry = float(m.group(3)) * beta, float(m.group(4)) * beta
                    rot = m.group(5)
                    large = m.group(6)
                    sweep = m.group(7)
                    x2, y2 = float(m.group(8)) * beta, float(m.group(9)) * beta
                    return f"M {m_x:.2f} {m_y:.2f} A {rx:.2f} {ry:.2f} {rot} {large} {sweep} {x2:.2f} {y2:.2f}"
                pattern = r"M\s*([-\d.]+)\s+([-\d.]+)\s+A\s*([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)"
                el.attrib['d'] = re.sub(pattern, repl_arc, el.attrib['d'])
                
            if 'data-arc' in el.attrib:
                parts = el.attrib['data-arc'].split(',')
                if len(parts) == 6:
                    try:
                        x, y, w, h, s, a = [float(p) for p in parts]
                        el.attrib['data-arc'] = f"{x*beta:.2f},{y*beta:.2f},{w*beta:.2f},{h*beta:.2f},{s:.2f},{a:.2f}"
                    except ValueError:
                        pass
                        
        elif tag == 'text':
            for a in ('x', 'y'):
                if a in el.attrib:
                    try:
                        val = float(el.attrib[a]) * beta
                        el.attrib[a] = f"{val:.2f}"
                    except ValueError:
                        pass
            if 'font-size' in el.attrib:
                try:
                    fs = float(el.attrib['font-size']) * beta
                    el.attrib['font-size'] = f"{min(fs, 20.0):.1f}"
                except ValueError:
                    pass
                    
    root.attrib['data-standard'] = '48px'
    
    xml_str = ET.tostring(root, encoding='utf-8').decode('utf-8')
    full_content = '<?xml version="1.0" encoding="UTF-8"?>\n' + xml_str + '\n'
    with open(path, 'w', encoding='utf-8') as f:
        f.write(full_content)
    return True

def update_catalog_metadata(catalog_path, base_dir):
    if not os.path.exists(catalog_path):
        print(f"Catálogo não encontrado em {catalog_path}")
        return
        
    print(f"Atualizando metadados em {catalog_path}...")
    with open(catalog_path, 'r', encoding='utf-8') as f:
        catalog = json.load(f)
        
    updated = 0
    for meta in catalog:
        cat = meta.get('category', '')
        svg_file = meta.get('svg_file', '')
        svg_path = os.path.join(base_dir, cat, svg_file)
        
        if not os.path.exists(svg_path):
            continue
            
        try:
            tree = ET.parse(svg_path)
            root = tree.getroot()
            
            if 'width' in root.attrib:
                meta['width'] = float(root.attrib['width'])
            if 'height' in root.attrib:
                meta['height'] = float(root.attrib['height'])
                
            if 'viewBox' in root.attrib:
                parts = root.attrib['viewBox'].split()
                if len(parts) == 4:
                    meta['hotspot_x'] = -float(parts[0])
                    meta['hotspot_y'] = -float(parts[1])
                    
            # Atualiza terminais
            terms = []
            for node in root.iter():
                if node.tag.endswith('circle') and 'qet-terminal' in node.attrib.get('class', ''):
                    name = node.attrib.get('data-name') or node.attrib.get('data-label') or ''
                    cx = float(node.attrib.get('cx', 0))
                    cy = float(node.attrib.get('cy', 0))
                    orient = node.attrib.get('data-orientation', 'n')
                    terms.append({
                        'name': name,
                        'x': cx,
                        'y': cy,
                        'orientation': orient
                    })
            meta['terminals'] = terms
            updated += 1
        except Exception as e:
            pass
            
    with open(catalog_path, 'w', encoding='utf-8') as f:
        json.dump(catalog, f, indent=2, ensure_ascii=False)
        
    print(f"Metadados de {updated} elementos atualizados com sucesso no catálogo!")

def main():
    base_all = 'symbols_qet_all'
    print("Normalizando todos os SVGs da biblioteca symbols_qet_all...")
    
    count = 0
    for root, dirs, files in os.walk(base_all):
        if 'cade_native' in root:
            continue
        for file in files:
            if file.endswith('.svg'):
                path = os.path.join(root, file)
                if scale_svg_file(path):
                    count += 1
                    if count % 1000 == 0:
                        print(f"  {count} SVGs processados...")
                        
    print(f"Total de {count} SVGs normalizados para a grade rígida de 48 px e bornes r=4.0 px!")
    
    # Atualiza catalog.json v1
    catalog_all = os.path.join(base_all, 'catalog.json')
    update_catalog_metadata(catalog_all, base_all)

if __name__ == '__main__':
    main()
