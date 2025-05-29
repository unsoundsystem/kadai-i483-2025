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
import org.apache.spark.streaming.kafka010.LocationStrategies.PreferConsistent
import org.apache.spark.streaming.kafka010.ConsumerStrategies.Subscribe
import org.apache.logging.log4j.Level
import org.apache.logging.log4j.core.config.Configurator

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
//        .option("topic", "i483-sensors-s2510150-analytics-s2510030_BH1750_avg-illumination")
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
//        //.option("topic", "i483-sensors-s2510150-co2_threshold-crossed")
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
    illumination_avg_stream.print()

    import java.util.Properties
    val props = new Properties()
    props.put("bootstrap.servers", "150.65.230.59:9092")
    props.put("key.serializer", "org.apache.kafka.common.serialization.StringSerializer")
    props.put("value.serializer", "org.apache.kafka.common.serialization.StringSerializer")

    illumination_avg_stream.foreachRDD { rdd =>
      rdd.foreachPartition { partitionOfRecords =>
        val producer = new KafkaProducer[String, String](props)
        partitionOfRecords.foreach { record =>
          val value = record.toString
          val message = new ProducerRecord[String, String]("i483-sensors-s2510030-BH1750_avg-illumination", value)
          producer.send(message)
        }
        producer.close()
      }
    }
    //val sum = stream //.map(record => s"Kafka Data: $record")
      //.map(r => (r.timestamp(), r.value()))
      //.window(Minutes(5), Seconds(30))
      //.reduceByKey(_ + _)

    //val count = stream
        //.map(r => (r.key(), r.value()))
        //.countByWindow(Minutes(5), Seconds(30))
    //val avg = count.cartesian(sum)
      //.map(r => r(1)(1).toDouble / r(1)(0))
    //avg.foreach(x => println(s"avg: $x"))
    ssc.start()
    ssc.awaitTermination()
  }
}

case class AvgState(count: Double = 0, sum: Double = 0) {
  val avg = sum / scala.math.max(count, 1)
}
