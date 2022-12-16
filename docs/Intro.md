iowatcher-ng is an executable for block I/O which consumes _blktrace_ kernel api.
The executable itself does not produce output such as _blkparse_ or _iowatcher_ but rather
exposes some of the same metrics to Prometheus which dials out to our exporter executable
to scrape them so you can use Grafana, for instance, to graph the values.