# dcos-thanos

A DC/OS framework for deploying a HA monitoring stack based on [Prometheus](https://prometheus.io/) and [Thanos](https://thanos.io).

Thanos provides components to store metrics data on external storage (object storage like S3) and provide a unified view on metrics collected by multiple prometheus instances.

This framework aims to provide a complete monitoring stack for DC/OS with prometheus, alertmanager, pushgateway and grafana.

## Getting started

### Requirements

* DC/OS >= 1.13
* At least 4 cores and 4 GB RAM free
* Thanos-supported object storage (see [thanos storage](https://thanos.io/tip/thanos/storage.md), if you have it available use S3 otherwise we recommend [https://min.io](MinIO))

### Provide package

Currently this package is not yet available from the DC/OS universe. As such you need to provide the package for your cluster yourself.

If you use the [DC/OS Package Registry](https://docs.d2iq.com/mesosphere/dcos/2.0/administering-clusters/package-registry/) you can download a bundle file from the github release page and upload it to your registry.

Otherwise you will have to build and upload the package yourself:

1. Install [dcosdev](https://github.com/swoehrl-mw/dcosdev) (fork with extra commands)
2. Install minio in your cluster
3. Set environment variables `MINIO_HOST`, `MINIO_ACCESS_KEY` and `MINIO_SECRET_KEY`
4. Run `dcosdev build java` (you need docker for that)
5. Download the files `fetcher` and `grafana-loader` from the github releases page and put them into the `files` folder
6. Run `dcosdev up`
7. Add the repo-file to your clusters package repositories (dcosdev will print the necessary command)
8. Now you can install the package (Note: Using this variant the version is always `snapshot`)

### Steps

1. Follow the [thanos storage documentation](https://thanos.io/tip/thanos/storage.md) and create your `bucket_config.yml` file. A minimal config for using minio could be:

    ```yaml
    type: S3
    config:
        bucket: "thanos-data"
        endpoint: "minio.marathon.l4lb.thisdcos.directory:9000"
        access_key: "minio"
        secret_key: "minio123"
        insecure: true
        signature_version2: false
    ```

2. Create a file `options.json` with at least the following content:

    ```json
    {
        "thanos": {
            "bucket_config_base64": "<base64-encoded-content-of-bucket_config.yml-file>",
        }
    }
    ```

3. Run `dcos package install thanos --options=options.json`

## Configuring thanos

Thanos provides a number of features that can be configured. They need to be added to the `options.json` file.

### Serviceaccount

if you are deploything thanos in a DC/OS cluster configured with strict security mode you need to provide a serviceaccount:

```json
{
    "service": {
        "service_account": "thanos-principal",
        "service_account_secret": "thanos/serviceaccount"
    }
}
```

To create the serviceaccount you can use the dcos cli with the following commands:

```bash
dcos security org service-accounts keypair private-key.pem public-key.pem
dcos security org service-accounts create -p public-key.pem -d "thanos service account" thanos-principal
dcos security secrets create-sa-secret --strict private-key.pem thanos-principal thanos/serviceaccount
dcos security org groups add_user superusers thanos-principal
```

### Pushgateway

This framework also includes a pushgateway instance. To enable it add the following option:

```json
{
    "pushgateway": {
        "enabled": true
    }
}
```

At the moment pushgateway can only be run as a single instance (no HA mode).

### HA

The monitoring stack can be run in a high availability manner. This includes:

* Multiple prometheus instances
* Multiple alertmanagers
* Multiple thanos store and query instances

To configure HA add the following options:

```json
{
    "prometheus": {
        "count": 2
    },
    "alertmanager": {
        "enabled": true,
        "ha": true
    },
    "thanos": {
        "query": {
            "count": 2
        },
        "store": {
            "count": 2
        }
    }
}
```

Notes:

* More than two prometheus instances are not recommened as each instance has to scrape each target and store each metric
* If you have a large number of queries against prometheus (e.g. from grafana) you can scale up the thanos query and store components to distribute load

### Custom grafana dashboards

Any dashboards you create within grafana are persisted across restarts. But if you ever need to issue a `pod replace` or the node grafana runs on fails these dashboards are lost. Therefore it is a good practice to store grafana dashboards in an external location and automatically load them into grafana. To do this you need to provide the dashboards as jsons in a zip file at a location where thanos can download it using http(s).

```json
{
    "grafana": {
        "dashboards": {
            "enabled": true,
            "url": "https://my.domain/dashboards.zip",
            "base_folder": "dashboards",
        }
    }
}
```

If you provide a `base_folder` only json files in that folder (and any subfolders) from the zip are used. Subfolders become folders in grafana (e.g. a file `dashboards/myapp/Overview.json` will be uploaded to grafana as a dashoard named `Overview` in a folder named `myapp`). Any files directly in the base folder will be put in the special `General` folder provided by grafana.

If you do not provide a dashboards url a set of default dashboards from [DC/OS grafana-dashboards](https://github.com/dcos/grafana-dashboards) (developed and provided by d2iq) are used. Note that these dashboards are for version 2.0.x of DC/OS. If you have a different version you should provide a url for those (select the branch for your version from [https://github.com/dcos/grafana-dashboards/branches](https://github.com/dcos/grafana-dashboards/branches) and select "Download ZIP"). In that case you also need to change `base_folder` to `grafana-dashboards-<version>/dashboards`.

### Grafana admin credentials (only for DC/OS EE)

You can configure custom grafana admin credentials by supplying secrets for them.

Run the following commands to create the secrets:

```bash
dcos security secrets create -v admin thanos/grafana-admin-username
dcos security secrets create -v mysupersecretpassword thanos/grafana-admin-password
```

Then add the following options:

```json
{
    "grafana": {
        "admin": {
            "username_secret": "thanos/grafana-admin-username",
            "password_secret": "thanos/grafana-admin-password"
        }
    }
}
```

### Custom grafana config

If you need more fine-grained control over grafana you can provide your own custom grafana.ini:

```json
{
    "grafana": {
        "config_base64": "<base64-encoded-content-of-grafana.ini>"
    }
}
```

### Alertingrules

To configure prometheus with alerting rules you need to put them in a zip file (no folders) and provide that zip file at a location where thanos can download it using http(s).

```json
{
    "prometheus": {
        "alerting_rules": {
            "url": "https://my.domain/alerting-rules.zip"
        }
    }
}
```

### Alertmanager

To configure the alertmanager you need to provide the config at a location where thanos can download it using http(s). This config can either be a single yaml file named `config.yml` or a zip file that must contain a file `config.yml`. Use the zip file if you need to provide e.g. templates.

```json
{
    "alertmanager": {
        "enabled": true,
        "config": {
            "url": "https://my.domain/alertmanager/config.yml"
        }
    }
}
```

Alternatively you can also provide the alertmanager config as a base64-encoded string:

```json
{
    "alertmanager": {
        "enabled": true,
        "config": {
            "content_base64": "<base64-encoded config.yml content>"
        }
    }
}
```

In this case any provided url will be ignored.

### Custom scrape targets

Prometheus will scrape all of its own components and the telegraf agent on each DC/OS node (formerly called dcos-metrics). The recommended way to get your own metrics into prometheus is to send them to telegraf (see [DC/OS metrics docs](https://docs.d2iq.com/mesosphere/dcos/2.1/metrics/)). In case that is not possible you can configure prometheus with additional scrape targets.
To do this create your list of [scrape configs](https://prometheus.io/docs/prometheus/latest/configuration/configuration/#scrape_config) as you would put them into the `prometheus.yml` file:

```yaml
  - job_name: 'myapp'
    static_configs:
      - targets: ['myapp.marathon.autoip.dcos.thisdcos.directory:80']
```

The list must be indented with exactly 2 spaces.

Then encode the config with base64 and put it into the options:

```json
{
    "prometheus": {
        "custom_scrape_config_base64": "<base64-encoded-scrape-config>"
    }
}
```

The framework will take that config and put it under the `scrape_configs` section of the `prometheus.yml`. if you use the wrong intendation or unknown configuration parameters the yml will become invalid and prometheus will fail to start (if that happens check the log of the prometheus instance for error messages).

### Thanos compactor and retention

As all data is stored on an external object storage the normal block compaction of prometheus can not happen. To work around this thanos includes a [compactor](https://thanos.io/tip/components/compact.md/) component that does compaction on blocks stored in object storage. This compactor is enabled by default. The compactor also does downsampling of older metrics for faster queries and handles retention:

```json
{
    "thanos": {
        "compact": {
            "downsampling_enabled": true,
            "retention": {
                "resolution_raw": "14d",
                "resolution_5m": "30d",
                "resolution_1h": "30d"
            }
        }
    }
}
```

if you don't need it you can also disable downsampling but this is not recommended. To disable retention set it to `0d`.

## Usage

### Grafana

If you want to access grafana outside of your cluster you will need to expose it through a loadbalancer (e.g. Edge-LB).

Unless you specified a different username / password in the options the default combination is `admin / admin`.

Grafana is automatically configured with a datasource pointing to the thanos query instances.

### Prometheus queries

If you want to directly query prometheus via its API you should not use the prometheus instances but instead use the thanos query instances as they provide a unified view of all metrics.

To connect to the query instances use the following endpoint from inside the cluster: `http://query.<servicename>.l4lb.thisdcos.directory:19192`.

### Push metrics to Pushgateway

If you have enabled it the [pushgateway](https://github.com/prometheus/pushgateway) is available inside the cluster under the endpoint `http://pushgateway.<servicename>.l4lb.thisdcos.directroy:9091`.

## Operations

### Structure / Components

For an idea on how a normal thanos architecture looks like see the [thanos architecture](https://github.com/thanos-io/thanos/blob/master/docs/img/arch.jpg).

This framework has the following components:

* Prometheus: One or more prometheus instances each with a thanos sidecar that takes care of pushing metric blocks to object storage
* Alertmanager: Optional, one or two instances configured as peers that handle alerting
* Pushgateway: Optional, one instance
* Thanos store: One or more instances, provides prometheus query api to query metrics in object storage
* Thanos query: One or more instances, provide the prometheus query api and collect metrics from all prometheus instances and stores
* Thanos compact: Optional, single instance, handles retention and downsampling of blocks in object storage
* Grafana: Optional, one instance, visualization frontend

### High availability

#### HA Prometheus

To make prometheus highly available you can configure more than one instance (see above). All instances will scrape all targets (so that means double the requests on targets) and store all metrics (that means double storage in the object storage). The idea is that if one instance fails the other is still active.
Additionally all metrics are stored on external object storage. This is done every two hours by the thanos sidecar (whenever prometheus finishes a tsdb block). If a prometheus instance fails all metrics collected since the last complete block are lost. But since there are at least two instances the other instance still has the metrics. The thanos query component handles deduplication so that metrics collected by both instances are only seen once in the result.

#### HA Alertmanager

In HA mode two alertmanager instances are started and configured to form a cluster. Prometheus will send all alerts to both instances and the alertmanager instances will handle deduplication. In case one instane fails the other can still handle alerts. During reconfiguration the framework will restart the instances one after the other so that at least one instance is running all the time.

### Reload dashboards

If you have configured a custom dashboards url (see above) you can reload the dashboards during runtime using the cli. To do this upload the zip with the new dashboards to the same location as the previous one and then simply run the following command: `dcos thanos plan start reload-grafana-dashboards`.

### Reload alertmanager config

You can reload the alertmanager config during runtime (unless you have provided it inline as a base64-encoded string). To this this upload the config (yml or zip) to the same location as the previous one and then simply run the following command: `dcos thanos plan start reload-alertmanager-config`.

### Reload alerting rules

You can reload the prometheus alerting rules during runtime. To this this upload the rules zip to the same location as the previous one and then simply run the following command: `dcos thanos plan start reload-prometheus-alert-rules`.

## Roadmap

* Support for Marathon-LB and/or DC/OS adminrouter for exposing grafana
* Support git for dashboard and alerting rules source
* Task to export grafana dashboards to s3/git
* Support for thanos receive

## Comparison with dcos-monitoring

This framework is an alternative to [dcos-monitoring](https://docs.d2iq.com/mesosphere/dcos/services/dcos-monitoring/), the official monitoring stack provided by d2iq. it has the following differences and similarities:

* Both provide a prometheus-based monitoring stack with prometheus, alertmanger, pushgateway and grafana
* dcos-thanos provides HA functionality whereas dcos-monitoring currently does not (unless you roll your own using remote write)
* dcos-monitoring needs to read alertingrules and grafana dashboards from a git repository, dcos-thanos can read them from any http endpoint

## Acknowledgments

This framework is inspired by [dcos-monitoring](https://docs.d2iq.com/mesosphere/dcos/services/dcos-monitoring/) from d2iq.

## Contributing

If you find a bug or have a feature request, please open an issue in Github. Or, if you want to contribute something, feel free to open a pull request.
