CREATE TABLE _rowva_records(id TEXT PRIMARY KEY,object_id TEXT NOT NULL REFERENCES _rowva_objects(id) ON DELETE CASCADE,revision INTEGER NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
CREATE INDEX _rowva_records_object ON _rowva_records(object_id);
CREATE TABLE _rowva_record_values(record_id TEXT NOT NULL REFERENCES _rowva_records(id) ON DELETE CASCADE,field_id TEXT NOT NULL REFERENCES _rowva_fields(id) ON DELETE CASCADE,value_json TEXT NOT NULL,PRIMARY KEY(record_id,field_id));
