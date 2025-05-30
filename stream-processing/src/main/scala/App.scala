import org.apache.spark._
import org.apache.spark.sql._
import org.apache.spark.sql.SparkSession
import org.apache.spark.sql.SparkSession._
//import org.apache.spark.sql.SparkSession.implicits._
import org.apache.spark.sql.functions._
import org.apache.spark.sql.types._
import org.apache.spark.sql.expressions.Window
import org.apache.kafka.clients.consumer.ConsumerConfig
import org.apache.kafka.common.serialization.StringDeserializer
import org.apache.kafka.clients.producer.{KafkaProducer, ProducerRecord}

import org.apache.spark.SparkConf
import org.apache.spark.streaming._
import org.apache.spark.streaming.kafka010._
import org.apache.kafka.clients.consumer.ConsumerRecord

import org.apache.spark.streaming.dstream.DStream
import org.apache.spark.streaming.kafka010.LocationStrategies.PreferConsistent
import org.apache.spark.streaming.kafka010.ConsumerStrategies.Subscribe
import org.apache.logging.log4j.Level
import org.apache.logging.log4j.core.config.Configurator
import org.apache.spark.streaming.State
import org.apache.spark.streaming.StateSpec

object App {
  def main(args: Array[String]): Unit = {
    println(classOf[org.apache.commons.lang3.SystemUtils].getResource("SystemUtils.class"))

    Configurator.setRootLevel(Level.WARN)

//    val spark = SparkSession.builder
//      .appName("Sensor data processing")
//      .master("local[*]")
//      .getOrCreate()
//    import spark.implicits._
//    val df = spark.readStream
//      .format("kafka")
//      .option("kafka.bootstrap.servers", "150.65.230.59:9092")
//      .option("subscribe", "i483-sensors-s2510030-BH1750-illumination")
//      .load;
//    val df_casted = df.select(
//      df("timestamp"),
//      df("value").cast("string").cast("double")
//    );
//
//      df.explain()
//      df_casted.explain()
//      val res = df_casted.groupBy(
//        window(df("timestamp"), "5 minutes", "30 seconds"),
//        df_casted("value"))
//        .mean("value")
//        .orderBy("window")
//        //println(res.toString);
//
//
//    val df_out = res.select(
//      res("value").cast("string")
//    );
//      val query = df_out.writeStream
//        .outputMode("complete")
//        .format("kafka")
//        .option("kafka.bootstrap.servers", "150.65.230.59:9092")
//        .option("checkpointLocation", "/tmp/i483-spark-checkpoints")
//        .option("topic", "i483-sensors--analytics-s2510030_BH1750_avg-illumination")
//        //.option("truncate", "false")
//        .start()
//      val query_debug = df_out.writeStream
//        .outputMode("complete")
//        .format("console")
//        .option("truncate", "false")
//        .start()
//
//
//    val co2_data = spark.readStream
//      .format("kafka")
//      .option("kafka.bootstrap.servers", "150.65.230.59:9092")
//      .option("subscribe", "i483-sensors-s2510030-SCD41-co2")
//      .load;
//
//    val co2_data_ = co2_data
//        .select(co2_data("value").cast("string").cast("double"))
//        .orderBy(co2_data("timestamp"))
//        .as[Double]
//        .rdd
//        .map(r => r > 700)
//        .aggregate(Seq[Boolean]())(
//          (acc: Seq[Boolean], x: Boolean) =>
//            if (acc.isEmpty || acc.last != x) {
//              acc ++ Seq(x)
//            } else {
//              acc
//            },
//          (acc1: Seq[Boolean], acc2: Seq[Boolean]) => acc1 ++ acc2
//        )
//
//    val schema = StructType(Seq(StructField("value", BooleanType, false)))
//    val td_query = spark.createDataFrame(spark.sparkContext.parallelize(co2_data_.map(b => Row(b))), schema)
//        .writeStream
//        .outputMode("complete")
//        .format("console")
//        //.format("kafka")
//        //.option("kafka.bootstrap.servers", "150.65.230.59:9092")
//        //.option("checkpointLocation", "/tmp/i483-spark-checkpoints")
//        //.option("topic", "i483-sensors--co2_threshold-crossed")
//        .start()
//
//      query.awaitTermination()
//      td_query.awaitTermination()
//      query_debug.awaitTermination()
      //td_query.awaitTermination()

    val spark = SparkSession.builder.appName("Sensor data processing")
      .master("local[*]")
      .getOrCreate()

    spark.sparkContext.setCheckpointDir("/tmp/i483-spark-checkpoints")
    val ssc = StreamingContext(spark.sparkContext, Seconds(2))

    val kafkaParams = Map[String, Object](
      "bootstrap.servers" -> "150.65.230.59:9092",
      "key.deserializer" -> classOf[StringDeserializer],
      "value.deserializer" -> classOf[StringDeserializer],
      "group.id" -> "KafkaData",
      "auto.offset.reset" -> "latest",
      "enable.auto.commit" -> "false",
    )

    val topics = Array("i483-sensors-s2510030-BH1750-illumination")
    val stream = KafkaUtils.createDirectStream[String, String](
      ssc,
      PreferConsistent,
      Subscribe[String, String](topics, kafkaParams)
    )
    //val update = (w: Seq[Double], state: Option[AvgState]) => {
      //val prev = state.getOrElse(AvgState())
      //val current = AvgState(prev.count + w.size, prev.sum + w.sum)
      //Some(current)
    //}
    val dstream_batch = stream.map(r => r.value().toDouble).window(Minutes(5), Seconds(30))
    val illumination_avg_stream = dstream_batch
      .transform { rdd =>
        if (rdd.isEmpty()) {
          ssc.sparkContext.emptyRDD[Double]
        } else {
          val sum = rdd.sum()
          val count = rdd.count()
          val avg = sum / count
          ssc.sparkContext.parallelize(Seq(avg))
        }
      }
      publish_kafka[Double](illumination_avg_stream, "i483-sensors-s2510030-BH1750_avg-illumination")

    val co2_stream = KafkaUtils.createDirectStream[String, String](
      ssc,
      PreferConsistent,
      Subscribe[String, String](
        Array("i483-sensors-reference-SCD4x_0_0x62-CO2")
        //Array("i483-sensors-s2510030-SCD41-co2")
        , kafkaParams)
    )
    val sf = StateSpec.function { (key: String, value: Option[Double], state: State[Option[Double]]) =>
          val prevOpt = state.getOption
          val current = value.get
          val is_crossed = prevOpt match {
            case Some(prev) =>
              if (prev.get <= 700 && current > 700) Some("yes")
              else if (prev.get > 700 && current <= 700) Some("no") else None
            case None => if (current > 700) Some("yes") else Some("no")
          }
          state.update(Some(current))
          is_crossed
        }
    val co2_threshold_yes_no = co2_stream
      .map(r => ("keeeeeeeeeeeeey", r.value().toDouble))
      .mapWithState(sf)
      .flatMap(_.toList)


    //co2_threshold_yes_no.print()
    publish_kafka[String](co2_threshold_yes_no, "i483-sensors-s2510030-co2_threshold-crossed")
    //co2_threshold_yes_no.foreachRDD { rdd =>
      //rdd.foreachPartition { partitionOfRecords =>
        //val producer = new KafkaProducer[String, String](props)
        //partitionOfRecords.foreach { record =>
          //val value = record.toString
          //val message = new ProducerRecord[String, String]("i483-sensors-s2510030-co2_threshold-crossed", value)
          //producer.send(message)
        //}
        //producer.close()
      //}
    //}


    // kadai 3
    val kadai3_topics = Array(
        "i483-sensors-s2510030-BH1750-illumination",
        "i483-sensors-s2510030-DPS310-air_pressure",
        "i483-sensors-s2510030-DPS310-temperature",
        "i483-sensors-s2510030-RPR0521-illumination",
        "i483-sensors-s2510030-RPR0521-infrared_illumination",
        "i483-sensors-s2510030-SCD41-co2",
        "i483-sensors-s2510030-SCD41-humidity",
        "i483-sensors-s2510030-SCD41-temperature"
      );
    val sensors_stream = KafkaUtils.createDirectStream[String, String](
      ssc,
      PreferConsistent,
      Subscribe[String, String](kadai3_topics, kafkaParams)
    )

    kadai3_topics.foreach { topic =>
      val out_topic_base =  "i483-sensors-s2510030-analytics" + topic.stripPrefix("i483-sensors-s2510030")
      val batch = sensors_stream
        .map(record => (record.topic, record.value().toDouble))
        .filter(r => r(0) == topic)
        .map(r => r(1))
        .window(Minutes(5), Seconds(30))
      // avg
      val avg = batch
        .transform { rdd =>
          if (rdd.isEmpty()) {
            ssc.sparkContext.emptyRDD[Double]
          } else {
            val sum = rdd.sum()
            val count = rdd.count()
            val avg = sum / count
            ssc.sparkContext.parallelize(Seq(avg))
          }
        }
      //avg.print()
      publish_kafka[Double](avg, out_topic_base + "-avg")
      // max
      val max = batch.reduce((x, y) => math.max(x, y))
      //max.print()
      publish_kafka[Double](max, out_topic_base + "-max")
      // min
      val min = batch.reduce((x, y) => math.min(x, y))
      //min.print()
      publish_kafka[Double](min, out_topic_base + "-min")
    }


    ssc.start()
    ssc.awaitTermination()
  }
}

case class AvgState(count: Double = 0, sum: Double = 0) {
  val avg = sum / scala.math.max(count, 1)
}

def publish_kafka[T](stream: DStream[T], topic: String): Unit = {
    import java.util.Properties
    val props = new Properties()
    props.put("bootstrap.servers", "150.65.230.59:9092")
    props.put("key.serializer", "org.apache.kafka.common.serialization.StringSerializer")
    props.put("value.serializer", "org.apache.kafka.common.serialization.StringSerializer")

    stream.foreachRDD { rdd =>
      rdd.foreachPartition { partitionOfRecords =>
        val producer = new KafkaProducer[String, String](props)
        partitionOfRecords.foreach { record =>
          val value = record.toString
          val message = new ProducerRecord[String, String](topic, value)
          producer.send(message)
        }
        producer.close()
      }
    }
}
