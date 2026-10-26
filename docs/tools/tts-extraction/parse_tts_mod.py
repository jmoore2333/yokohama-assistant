#!/usr/bin/env python3
"""Parse TTS mod JSON to extract asset mappings."""

import json
import re
from pathlib import Path
from collections import defaultdict

def url_to_filename(url):
    """Convert URL to TTS cache filename format."""
    return re.sub(r'[^a-zA-Z0-9]', '', url) + ('.jpg' if '.jpg' in url.lower() else '.png')

def extract_objects(data, path=""):
    """Recursively extract all objects with their image URLs."""
    objects = []

    if isinstance(data, dict):
        obj = {}

        # Get object identifiers
        if 'Name' in data:
            obj['type'] = data.get('Name', '')
        if 'Nickname' in data and data['Nickname']:
            obj['nickname'] = data['Nickname']
        if 'Description' in data and data['Description']:
            obj['description'] = data['Description']
        if 'GUID' in data:
            obj['guid'] = data['GUID']

        # Get image URLs
        image_urls = {}
        for key in ['FaceURL', 'BackURL', 'ImageURL', 'DiffuseURL', 'NormalURL']:
            if key in data and data[key]:
                image_urls[key] = data[key]

        # CustomDeck contains card sheet info
        if 'CustomDeck' in data:
            for deck_id, deck_info in data['CustomDeck'].items():
                if 'FaceURL' in deck_info:
                    image_urls[f'Deck{deck_id}_Face'] = deck_info['FaceURL']
                if 'BackURL' in deck_info:
                    image_urls[f'Deck{deck_id}_Back'] = deck_info['BackURL']
                if 'NumWidth' in deck_info:
                    obj[f'deck{deck_id}_cols'] = deck_info['NumWidth']
                if 'NumHeight' in deck_info:
                    obj[f'deck{deck_id}_rows'] = deck_info['NumHeight']

        if image_urls:
            obj['images'] = image_urls

        if obj.get('type') or obj.get('nickname') or obj.get('images'):
            obj['path'] = path
            objects.append(obj)

        # Recurse into child objects
        for key, value in data.items():
            if key in ['ObjectStates', 'ContainedObjects', 'States']:
                child_path = f"{path}/{key}" if path else key
                objects.extend(extract_objects(value, child_path))
            elif isinstance(value, (dict, list)):
                objects.extend(extract_objects(value, path))

    elif isinstance(data, list):
        for i, item in enumerate(data):
            objects.extend(extract_objects(item, f"{path}[{i}]"))

    return objects

def main():
    mod_path = Path(__file__).parent / "1840951801.json"

    with open(mod_path, 'r') as f:
        mod_data = json.load(f)

    # Extract all objects
    objects = extract_objects(mod_data)

    # Group by type and collect unique image URLs
    by_type = defaultdict(list)
    all_urls = {}

    for obj in objects:
        if obj.get('type'):
            by_type[obj['type']].append(obj)

        if 'images' in obj:
            for url_type, url in obj['images'].items():
                if url not in all_urls:
                    all_urls[url] = {
                        'url_type': url_type,
                        'objects': []
                    }
                all_urls[url]['objects'].append({
                    'type': obj.get('type', ''),
                    'nickname': obj.get('nickname', ''),
                    'guid': obj.get('guid', ''),
                    'deck_info': {k: v for k, v in obj.items() if 'deck' in k}
                })

    # Print summary
    print("=" * 60)
    print("OBJECT TYPES")
    print("=" * 60)
    for obj_type, objs in sorted(by_type.items(), key=lambda x: -len(x[1])):
        nicknames = set(o.get('nickname', '') for o in objs if o.get('nickname'))
        print(f"\n{obj_type}: {len(objs)} objects")
        if nicknames:
            print(f"  Named: {', '.join(sorted(nicknames))}")

    print("\n" + "=" * 60)
    print("IMAGE URL MAPPING")
    print("=" * 60)

    for url, info in sorted(all_urls.items()):
        filename = url_to_filename(url)
        obj_info = info['objects'][0] if info['objects'] else {}
        print(f"\n{info['url_type']}:")
        print(f"  URL: {url}")
        print(f"  File: {filename}")
        print(f"  Used by: {obj_info.get('type', '?')} - {obj_info.get('nickname', '(unnamed)')}")
        if obj_info.get('deck_info'):
            print(f"  Deck: {obj_info['deck_info']}")

    # Save mapping to JSON
    output = {
        'url_mapping': {url: {'filename': url_to_filename(url), **info} for url, info in all_urls.items()},
        'objects_by_type': {t: [{'nickname': o.get('nickname'), 'guid': o.get('guid')} for o in objs] for t, objs in by_type.items()}
    }

    with open(Path(__file__).parent / 'asset_mapping.json', 'w') as f:
        json.dump(output, f, indent=2)

    print(f"\n\nSaved mapping to asset_mapping.json")

if __name__ == '__main__':
    main()
