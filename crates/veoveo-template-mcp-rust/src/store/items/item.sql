-- One item of a tenant by its key.
SELECT item_id AS "item_id: ItemId", name
FROM template.item
WHERE tenant = $1 AND item_id = $2
