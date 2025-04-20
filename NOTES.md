# ベクトルタイル化に関するメモ

## 概要

1. `dl-tool` すべてのZIPファイルのダウンロード
2. `dl-tool` ZIPからFGBに変換
3. `dl-tool` FGBから並列処理を使ってPostGISにインポート
4. `dl-tool` PostGISから1つのFGBに書き出し
5. `tippecanoe` FGBから最高解像度レベルのタイルを作成 (z16)
6. `postgis` 地域メッシュのアグリゲーションデータ作成
7. `ogr2ogr` アグリゲーションデータからFGB書き出し
8. `tippecanoe` アグリゲーションのタイル化
9. `tile-join` アーカイブ結合

## コマンド類

### 6. 地域メッシュのアグリゲーション化

**4次メッシュ**

```sql
CREATE TABLE grid_lv4 AS
WITH lv1_grid AS (
  SELECT
    meshcode,
    jismesh.to_meshpoly_geom(meshcode) as geom
  FROM jismesh.japan_lv1_meshes
)
SELECT mesh.meshcode, mesh.geom
FROM lv1_grid
CROSS JOIN LATERAL
  jismesh.to_meshcodes(lv1_grid.geom,'Lv4') AS mesh(meshcode,geom);
CREATE INDEX ON grid_lv4 USING GIST(geom);

create unlogged table mojxml_lv4 as
SELECT
  COUNT(x.*) as "count",
  grid.geom
FROM mojxml x
INNER JOIN grid_lv4 grid ON grid.geom && x.wkb_geometry
GROUP BY grid.geom
HAVING COUNT(x.*) > 0
```

**3次メッシュ**

3次からは総数少ないので補助テーブル作る必要ない

```sql
create unlogged table mojxml_lv3 as
WITH lv1_grid AS (
  SELECT
    meshcode,
    jismesh.to_meshpoly_geom(meshcode) as geom
  FROM jismesh.japan_lv1_meshes
),
grid AS (
SELECT
lv3.meshcode,
lv3.geom
FROM lv1_grid lv1
CROSS JOIN LATERAL jismesh.to_meshcodes(lv1.geom, 'Lv3'::jismesh.mesh_level) AS lv3(meshcode, geom)
)
SELECT
  COUNT(x.*) as "count",
  grid.geom
FROM mojxml x
INNER JOIN grid ON grid.geom && x.wkb_geometry
GROUP BY grid.geom
HAVING COUNT(x.*) > 0
```

**2次メッシュ**

```sql
create unlogged table mojxml_lv2 as
WITH lv1_grid AS (
  SELECT
    meshcode,
    jismesh.to_meshpoly_geom(meshcode) as geom
  FROM jismesh.japan_lv1_meshes
),
grid AS (
SELECT
lv2.meshcode,
lv2.geom
FROM lv1_grid lv1
CROSS JOIN LATERAL jismesh.to_meshcodes(lv1.geom, 'Lv2'::jismesh.mesh_level) AS lv2(meshcode, geom)
)
SELECT
  COUNT(x.*) as "count",
  grid.geom
FROM mojxml x
INNER JOIN grid ON grid.geom && x.wkb_geometry
GROUP BY grid.geom
HAVING COUNT(x.*) > 0
```

**1次メッシュ**

```sql
create unlogged table mojxml_lv1 as
WITH grid AS (
  SELECT
    meshcode,
    jismesh.to_meshpoly_geom(meshcode) as geom
  FROM jismesh.japan_lv1_meshes
)
SELECT
  COUNT(x.*) as "count",
  grid.geom
FROM mojxml x
INNER JOIN grid ON grid.geom && x.wkb_geometry
GROUP BY grid.geom
HAVING COUNT(x.*) > 0
```

### 7. アグリゲーションデータの書き出し

```shell
ogr2ogr -f FlatGeobuf -overwrite ./mojxml_lv4.fgb PG:"$PG_CONN_STR" mojxml_lv4 -t_srs EPSG:4326 -lco SPATIAL_INDEX=NO
ogr2ogr -f FlatGeobuf -overwrite ./mojxml_lv3.fgb PG:"$PG_CONN_STR" mojxml_lv3 -t_srs EPSG:4326 -lco SPATIAL_INDEX=NO
ogr2ogr -f FlatGeobuf -overwrite ./mojxml_lv2.fgb PG:"$PG_CONN_STR" mojxml_lv2 -t_srs EPSG:4326 -lco SPATIAL_INDEX=NO
ogr2ogr -f FlatGeobuf -overwrite ./mojxml_lv1.fgb PG:"$PG_CONN_STR" mojxml_lv1 -t_srs EPSG:4326 -lco SPATIAL_INDEX=NO
```

### 8. アグリゲーションデータのタイル化

```shell
tippecanoe -o "mojxml_lv1.mbtiles" -Z0 -z4 --no-simplification-of-shared-nodes --layer=mojxml_agg "./mojxml_lv1.fgb"
tippecanoe -o "mojxml_lv2.mbtiles" -Z5 -z8 --no-simplification-of-shared-nodes --layer=mojxml_agg "./mojxml_lv2.fgb"
tippecanoe -o "mojxml_lv3.mbtiles" -Z9 -z10 --no-simplification-of-shared-nodes --layer=mojxml_agg "./mojxml_lv3.fgb"
tippecanoe -o "mojxml_lv4.mbtiles" -Z11 -z15 --no-simplification-of-shared-nodes --layer=mojxml_agg "./mojxml_lv4.fgb"
```

### 9. タイル結合

```shell
tile-join -o mojxml_2025.pmtiles \
  --force \
  "mojxml_lv1.mbtiles" \
  "mojxml_lv2.mbtiles" \
  "mojxml_lv3.mbtiles" \
  "mojxml_lv4.mbtiles" \
  "merged.pmtiles"
```
