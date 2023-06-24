CREATE TYPE "Job_Status_Type" AS ENUM ('queued', 'running', 'completed', 'failed');
CREATE TABLE jobs (
  id SERIAL PRIMARY KEY,
  status "Job_Status_Type" NOT NULL,
  date_submitted TIMESTAMP NOT NULL,
  date_started TIMESTAMP,
  date_finished TIMESTAMP,
  query BYTEA NOT NULL,
  response BYTEA
);
